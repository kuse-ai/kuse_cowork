use calamine::{open_workbook_auto, Data, Range, Reader, Sheets};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::types::*;

/// Default maximum rows to read in a single request
const DEFAULT_MAX_ROWS: u32 = 1000;

/// Read Excel file with pagination support
pub fn read_excel(options: &ExcelReadOptions) -> Result<ExcelReadResult, ExcelError> {
    let path = Path::new(&options.path);

    if !path.exists() {
        return Err(ExcelError::file_not_found(&options.path));
    }

    // Compute checksum first
    let checksum = compute_checksum(&options.path)?;

    // Open the workbook
    let mut workbook: Sheets<_> = open_workbook_auto(&options.path)
        .map_err(|e| ExcelError::read_error(format!("Failed to open workbook: {}", e)))?;

    // Get sheet names and info
    let sheet_names = workbook.sheet_names().to_vec();
    let mut sheets: Vec<SheetInfo> = Vec::new();

    for (index, name) in sheet_names.iter().enumerate() {
        if let Ok(range) = workbook.worksheet_range(name) {
            let (rows, cols) = range.get_size();
            sheets.push(SheetInfo {
                name: name.clone(),
                index: index as u32,
                row_count: rows as u32,
                col_count: cols as u32,
            });
        }
    }

    // Determine which sheet to read
    let target_sheet = options.sheet.clone().unwrap_or_else(|| {
        sheet_names.first().cloned().unwrap_or_default()
    });

    // Get the sheet range
    let range = workbook.worksheet_range(&target_sheet)
        .map_err(|e| ExcelError::read_error(format!("Failed to read sheet '{}': {}", target_sheet, e)))?;

    let (total_row_count, col_count) = range.get_size();
    let total_rows = total_row_count as u32;

    // Apply offset and limit
    let offset = options.offset.unwrap_or(0);
    let max_rows = options.max_rows.unwrap_or(DEFAULT_MAX_ROWS);

    // Read rows with pagination
    let (rows, columns) = read_range_paginated(&range, offset, max_rows)?;

    let has_more = offset + (rows.len() as u32) < total_rows;

    // Get file size
    let file_size = std::fs::metadata(&options.path)
        .map(|m| m.len())
        .unwrap_or(0);

    let size_info = SizeInfo {
        file_size_bytes: file_size,
        row_count: total_rows,
        col_count: col_count as u32,
    };

    // TODO: Parse named ranges if needed
    let named_ranges = Vec::new();

    Ok(ExcelReadResult {
        sheets,
        named_ranges,
        rows,
        columns,
        checksum,
        size_info,
        total_rows,
        has_more,
        offset,
    })
}

/// Read a range with pagination
fn read_range_paginated(
    range: &Range<Data>,
    offset: u32,
    max_rows: u32,
) -> Result<(Vec<Vec<CellValue>>, Vec<ColumnInfo>), ExcelError> {
    let (row_count, col_count) = range.get_size();
    let start = range.start().unwrap_or((0, 0));

    let mut rows = Vec::new();
    let mut columns = Vec::new();
    let mut header_row: Option<Vec<String>> = None;

    // Build column info from first row (assumed header) or column letters
    for col_idx in 0..col_count {
        let col_letter = column_index_to_letter(col_idx as u32);
        columns.push(ColumnInfo {
            index: col_idx as u32,
            name: col_letter,
            header: None,
            width: None,
            data_type: None,
        });
    }

    // Read rows with pagination
    let start_row = offset as usize;
    let end_row = std::cmp::min(start_row + max_rows as usize, row_count);

    // Type inference tracking
    let mut col_type_counts: HashMap<usize, HashMap<&str, u32>> = HashMap::new();

    for row_idx in start_row..end_row {
        let mut row_data = Vec::new();

        for col_idx in 0..col_count {
            let cell = range.get((row_idx, col_idx));
            let cell_value = convert_cell_value(cell);

            // Track types for inference
            let type_str = match &cell_value {
                CellValue::Empty => "empty",
                CellValue::String(_) => "string",
                CellValue::Number(_) => "number",
                CellValue::Boolean(_) => "boolean",
                CellValue::DateTime(_) => "date",
                CellValue::Error(_) => "error",
                CellValue::Formula { .. } => "formula",
            };

            col_type_counts
                .entry(col_idx)
                .or_default()
                .entry(type_str)
                .and_modify(|c| *c += 1)
                .or_insert(1);

            // Capture header row (first row if offset is 0)
            if row_idx == 0 && offset == 0 {
                if header_row.is_none() {
                    header_row = Some(Vec::new());
                }
                if let Some(ref mut headers) = header_row {
                    let header_text = match &cell_value {
                        CellValue::String(s) => s.clone(),
                        CellValue::Number(n) => n.to_string(),
                        _ => String::new(),
                    };
                    headers.push(header_text);
                }
            }

            row_data.push(cell_value);
        }

        rows.push(row_data);
    }

    // Update column info with headers and inferred types
    if let Some(headers) = header_row {
        for (idx, header) in headers.into_iter().enumerate() {
            if idx < columns.len() {
                columns[idx].header = if header.is_empty() { None } else { Some(header) };
            }
        }
    }

    // Infer column types
    for (col_idx, type_counts) in col_type_counts {
        if col_idx < columns.len() {
            let dominant_type = type_counts
                .into_iter()
                .filter(|(t, _)| *t != "empty")
                .max_by_key(|(_, count)| *count)
                .map(|(t, _)| t)
                .unwrap_or("string");

            columns[col_idx].data_type = Some(dominant_type.to_string());
        }
    }

    Ok((rows, columns))
}

/// Convert calamine Data to our CellValue
fn convert_cell_value(cell: Option<&Data>) -> CellValue {
    match cell {
        None => CellValue::Empty,
        Some(data) => match data {
            Data::Empty => CellValue::Empty,
            Data::String(s) => CellValue::String(s.clone()),
            Data::Float(f) => CellValue::Number(*f),
            Data::Int(i) => CellValue::Number(*i as f64),
            Data::Bool(b) => CellValue::Boolean(*b),
            Data::DateTime(dt) => {
                // Convert Excel datetime to ISO string using calamine's as_f64
                let f = dt.as_f64();
                CellValue::DateTime(format_excel_datetime(f))
            }
            Data::DateTimeIso(s) => CellValue::DateTime(s.clone()),
            Data::DurationIso(s) => CellValue::String(s.clone()),
            Data::Error(e) => CellValue::Error(format!("{:?}", e)),
        },
    }
}

/// Format Excel datetime (days since 1899-12-30) to ISO 8601
fn format_excel_datetime(value: f64) -> String {
    // Excel epoch is December 30, 1899
    let days = value.floor() as i64;
    let time_fraction = value.fract();

    // Calculate date
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    let date = epoch + chrono::Duration::days(days);

    // Calculate time
    let total_seconds = (time_fraction * 86400.0).round() as u32;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let time = chrono::NaiveTime::from_hms_opt(hours, minutes, seconds).unwrap_or_default();
    let datetime = chrono::NaiveDateTime::new(date, time);

    datetime.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Convert column index (0-based) to Excel column letter (A, B, ..., Z, AA, AB, ...)
fn column_index_to_letter(index: u32) -> String {
    let mut result = String::new();
    let mut n = index + 1;

    while n > 0 {
        n -= 1;
        let c = (b'A' + (n % 26) as u8) as char;
        result.insert(0, c);
        n /= 26;
    }

    result
}

/// Get list of sheets in a workbook
pub fn get_sheets(path: &str) -> Result<Vec<SheetInfo>, ExcelError> {
    let workbook: Sheets<_> = open_workbook_auto(path)
        .map_err(|e| ExcelError::read_error(format!("Failed to open workbook: {}", e)))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut sheets = Vec::new();

    for (index, name) in sheet_names.iter().enumerate() {
        // We can't easily get row/col count without reading the sheet
        sheets.push(SheetInfo {
            name: name.clone(),
            index: index as u32,
            row_count: 0,
            col_count: 0,
        });
    }

    Ok(sheets)
}

/// Compute SHA-256 checksum of a file
pub fn compute_checksum(path: &str) -> Result<String, ExcelError> {
    let mut file = File::open(path)
        .map_err(|e| ExcelError::read_error(format!("Failed to open file for checksum: {}", e)))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)
            .map_err(|e| ExcelError::read_error(format!("Failed to read file for checksum: {}", e)))?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_index_to_letter() {
        assert_eq!(column_index_to_letter(0), "A");
        assert_eq!(column_index_to_letter(1), "B");
        assert_eq!(column_index_to_letter(25), "Z");
        assert_eq!(column_index_to_letter(26), "AA");
        assert_eq!(column_index_to_letter(27), "AB");
        assert_eq!(column_index_to_letter(51), "AZ");
        assert_eq!(column_index_to_letter(52), "BA");
    }
}
