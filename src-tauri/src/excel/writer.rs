use umya_spreadsheet::{self, Spreadsheet, Worksheet, new_file, reader, writer};
use std::path::Path;

use super::reader::compute_checksum;
use super::types::{self, ApplyResult, CellEdit, CellValue, ExcelError, ExcelErrorType, ColumnInfo};

/// Apply edits to an Excel file, preserving formulas and formatting
pub fn apply_edits(
    path: &str,
    sheet: &str,
    edits: &[CellEdit],
    validate_checksum: Option<&str>,
) -> Result<ApplyResult, ExcelError> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        return Err(ExcelError::file_not_found(path));
    }

    // Check checksum if provided
    if let Some(expected_checksum) = validate_checksum {
        let current_checksum = compute_checksum(path)?;
        if current_checksum != expected_checksum {
            return Err(ExcelError::new(
                format!(
                    "File has been modified. Expected checksum: {}, current: {}",
                    expected_checksum, current_checksum
                ),
                ExcelErrorType::ChecksumMismatch,
            ));
        }
    }

    // Open the workbook
    let mut book = reader::xlsx::read(file_path)
        .map_err(|e| ExcelError::read_error(format!("Failed to open workbook: {}", e)))?;

    // Find the sheet
    let worksheet = book.get_sheet_by_name_mut(sheet)
        .ok_or_else(|| ExcelError::sheet_not_found(sheet))?;

    let mut edits_applied = 0;
    let mut errors = Vec::new();

    // Apply each edit
    for edit in edits {
        match apply_single_edit(worksheet, edit) {
            Ok(_) => edits_applied += 1,
            Err(e) => errors.push(format!(
                "Row {}, Col {}: {}",
                edit.row, edit.col, e
            )),
        }
    }

    // Update metadata if we have a Kuse metadata sheet
    update_kuse_metadata(&mut book);

    // Save the workbook
    writer::xlsx::write(&book, file_path)
        .map_err(|e| ExcelError::write_error(format!("Failed to save workbook: {}", e)))?;

    // Compute new checksum
    let new_checksum = compute_checksum(path)?;

    Ok(ApplyResult {
        success: errors.is_empty(),
        edits_applied,
        new_checksum,
        errors,
    })
}

/// Apply a single cell edit
fn apply_single_edit(
    worksheet: &mut Worksheet,
    edit: &CellEdit,
) -> Result<(), String> {
    // Excel uses 1-based indexing
    let row_num = edit.row + 1;
    let col_num = edit.col + 1;

    let cell = worksheet.get_cell_mut((col_num, row_num));

    match &edit.value {
        CellValue::Empty => {
            cell.set_value("");
        }
        CellValue::String(s) => {
            cell.set_value(s);
        }
        CellValue::Number(n) => {
            // Convert number to string for umya-spreadsheet
            cell.set_value(n.to_string());
        }
        CellValue::Boolean(b) => {
            // Convert boolean to string
            cell.set_value(if *b { "TRUE" } else { "FALSE" });
        }
        CellValue::DateTime(dt) => {
            // Try to parse and set as datetime
            cell.set_value(dt);
        }
        CellValue::Error(e) => {
            // Can't really set an error value, so set as string
            cell.set_value(format!("#{}", e));
        }
        CellValue::Formula { formula, .. } => {
            // Set the formula (without leading =, umya adds it)
            let formula_text = formula.strip_prefix('=').unwrap_or(formula);
            cell.set_formula(formula_text);
        }
    }

    Ok(())
}

/// Update Kuse metadata sheet with version info
fn update_kuse_metadata(book: &mut Spreadsheet) {
    const METADATA_SHEET: &str = "_KuseMetadata";

    // Check if metadata sheet exists
    let has_sheet = book.get_sheet_by_name(METADATA_SHEET).is_some();

    if !has_sheet {
        // Create metadata sheet
        let _ = book.new_sheet(METADATA_SHEET);
    }

    if let Some(sheet) = book.get_sheet_by_name_mut(METADATA_SHEET) {
        // Set metadata values
        let now = chrono::Utc::now().to_rfc3339();

        sheet.get_cell_mut("A1").set_value("Property");
        sheet.get_cell_mut("B1").set_value("Value");

        sheet.get_cell_mut("A2").set_value("LastModified");
        sheet.get_cell_mut("B2").set_value(&now);

        sheet.get_cell_mut("A3").set_value("ModifiedBy");
        sheet.get_cell_mut("B3").set_value("Kuse Cowork");

        sheet.get_cell_mut("A4").set_value("Version");
        sheet.get_cell_mut("B4").set_value("1.0");
    }
}

/// Create a backup of the file before editing
pub fn create_backup(path: &str) -> Result<String, ExcelError> {
    let file_path = Path::new(path);

    if !file_path.exists() {
        return Err(ExcelError::file_not_found(path));
    }

    let backup_name = format!(
        "{}.backup.{}",
        path,
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    std::fs::copy(path, &backup_name)
        .map_err(|e| ExcelError::write_error(format!("Failed to create backup: {}", e)))?;

    Ok(backup_name)
}

/// Export data to a new Excel file
pub fn export_to_new_file(
    rows: &[Vec<CellValue>],
    columns: &[ColumnInfo],
    output_path: &str,
    sheet_name: Option<&str>,
) -> Result<String, ExcelError> {
    let mut book = new_file();

    let sheet_name = sheet_name.unwrap_or("Sheet1");

    // Create sheet if it doesn't exist
    if book.get_sheet_by_name(sheet_name).is_none() {
        let _ = book.new_sheet(sheet_name);
    }

    // Get the sheet (guaranteed to exist now)
    let sheet = book.get_sheet_by_name_mut(sheet_name)
        .expect("Sheet should exist after creation");

    // Write header row if we have column headers
    let has_headers = columns.iter().any(|c| c.header.is_some());
    let data_start_row = if has_headers { 2u32 } else { 1u32 };

    if has_headers {
        for (col_idx, col) in columns.iter().enumerate() {
            let col_num = (col_idx + 1) as u32;
            if let Some(ref header) = col.header {
                sheet.get_cell_mut((col_num, 1)).set_value(header);
            }
        }
    }

    // Write data rows
    for (row_idx, row) in rows.iter().enumerate() {
        let row_num = data_start_row + row_idx as u32;

        for (col_idx, cell) in row.iter().enumerate() {
            let col_num = (col_idx + 1) as u32;
            let cell_ref = sheet.get_cell_mut((col_num, row_num));

            match cell {
                CellValue::Empty => {}
                CellValue::String(s) => {
                    cell_ref.set_value(s);
                }
                CellValue::Number(n) => {
                    cell_ref.set_value(n.to_string());
                }
                CellValue::Boolean(b) => {
                    cell_ref.set_value(if *b { "TRUE" } else { "FALSE" });
                }
                CellValue::DateTime(dt) => {
                    cell_ref.set_value(dt);
                }
                CellValue::Error(e) => {
                    cell_ref.set_value(format!("#{}", e));
                }
                CellValue::Formula { formula, .. } => {
                    let formula_text = formula.strip_prefix('=').unwrap_or(formula);
                    cell_ref.set_formula(formula_text);
                }
            }
        }
    }

    // Save the file
    writer::xlsx::write(&book, output_path)
        .map_err(|e| ExcelError::write_error(format!("Failed to write file: {}", e)))?;

    // Compute checksum of new file
    compute_checksum(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require actual Excel files
}
