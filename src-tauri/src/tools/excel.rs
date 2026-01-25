use crate::agent::ToolDefinition;
use crate::excel::{self, CellValue, ExcelReadOptions};
use serde_json::json;

/// Excel filter tool definition
pub fn filter_definition() -> ToolDefinition {
    ToolDefinition {
        name: "excel_filter".to_string(),
        description: "Filter rows from an Excel file based on column conditions. Returns matching rows as a formatted table. Use the file_path from the Excel Data Context.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Full path to the Excel file (from Excel Data Context)"
                },
                "sheet": {
                    "type": "string",
                    "description": "Sheet name (optional, uses first sheet if not specified)"
                },
                "column": {
                    "type": "string",
                    "description": "Column name or index (A, B, C, etc.) to filter on"
                },
                "operator": {
                    "type": "string",
                    "enum": ["equals", "not_equals", "contains", "not_contains", "greater_than", "less_than", "greater_equal", "less_equal", "is_empty", "not_empty"],
                    "description": "Comparison operator"
                },
                "value": {
                    "type": "string",
                    "description": "Value to compare against (not needed for is_empty/not_empty)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of rows to return (default: 100)"
                }
            },
            "required": ["file_path", "column", "operator"]
        }),
    }
}

/// Excel search tool definition
pub fn search_definition() -> ToolDefinition {
    ToolDefinition {
        name: "excel_search".to_string(),
        description: "Search for a value across columns in an Excel file. Returns rows containing the search term. Use the file_path from the Excel Data Context.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Full path to the Excel file (from Excel Data Context)"
                },
                "sheet": {
                    "type": "string",
                    "description": "Sheet name (optional, uses first sheet if not specified)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query (case-insensitive)"
                },
                "columns": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Optional: specific columns to search in (default: all columns)"
                },
                "exact_match": {
                    "type": "boolean",
                    "description": "If true, requires exact cell value match (default: false, uses contains)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of rows to return (default: 50)"
                }
            },
            "required": ["file_path", "query"]
        }),
    }
}

/// Excel aggregate tool definition
pub fn aggregate_definition() -> ToolDefinition {
    ToolDefinition {
        name: "excel_aggregate".to_string(),
        description: "Compute aggregate statistics on numeric columns in an Excel file. Use the file_path from the Excel Data Context.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Full path to the Excel file (from Excel Data Context)"
                },
                "sheet": {
                    "type": "string",
                    "description": "Sheet name (optional, uses first sheet if not specified)"
                },
                "column": {
                    "type": "string",
                    "description": "Column name or index to aggregate"
                },
                "operations": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["sum", "avg", "min", "max", "count", "count_distinct", "median"]
                    },
                    "description": "Aggregation operations to perform (default: all)"
                },
                "group_by": {
                    "type": "string",
                    "description": "Optional column to group by before aggregating"
                }
            },
            "required": ["file_path", "column"]
        }),
    }
}

/// Get all Excel tool definitions
pub fn get_excel_tools() -> Vec<ToolDefinition> {
    vec![
        filter_definition(),
        search_definition(),
        aggregate_definition(),
    ]
}

/// Execute excel_filter tool
pub fn execute_filter(
    input: &serde_json::Value,
    file_path: &str,
    sheet: Option<&str>,
) -> Result<String, String> {
    let column = input
        .get("column")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'column' parameter")?;

    let operator = input
        .get("operator")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'operator' parameter")?;

    let value = input.get("value").and_then(|v| v.as_str()).unwrap_or("");

    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    // Read Excel data
    let options = ExcelReadOptions {
        path: file_path.to_string(),
        sheet: sheet.map(|s| s.to_string()),
        range: None,
        offset: None,
        max_rows: Some(10000), // Read up to 10k rows
    };

    let result = excel::read_excel(&options)
        .map_err(|e| format!("Failed to read Excel: {}", e))?;

    // Find column index
    let col_idx = find_column_index(&result.columns, column)?;

    // Filter rows
    let mut filtered_rows: Vec<(usize, &Vec<CellValue>)> = Vec::new();

    for (row_idx, row) in result.rows.iter().enumerate() {
        if filtered_rows.len() >= limit {
            break;
        }

        let cell = row.get(col_idx).cloned().unwrap_or(CellValue::Empty);
        let cell_str = cell_to_string(&cell);

        let matches = match operator {
            "equals" => cell_str.eq_ignore_ascii_case(value),
            "not_equals" => !cell_str.eq_ignore_ascii_case(value),
            "contains" => cell_str.to_lowercase().contains(&value.to_lowercase()),
            "not_contains" => !cell_str.to_lowercase().contains(&value.to_lowercase()),
            "greater_than" => compare_numeric(&cell, value, |a, b| a > b),
            "less_than" => compare_numeric(&cell, value, |a, b| a < b),
            "greater_equal" => compare_numeric(&cell, value, |a, b| a >= b),
            "less_equal" => compare_numeric(&cell, value, |a, b| a <= b),
            "is_empty" => matches!(cell, CellValue::Empty) || cell_str.trim().is_empty(),
            "not_empty" => !matches!(cell, CellValue::Empty) && !cell_str.trim().is_empty(),
            _ => return Err(format!("Unknown operator: {}", operator)),
        };

        if matches {
            filtered_rows.push((result.offset as usize + row_idx + 1, row));
        }
    }

    // Format output as markdown table
    format_table_output(&result.columns, &filtered_rows, limit)
}

/// Execute excel_search tool
pub fn execute_search(
    input: &serde_json::Value,
    file_path: &str,
    sheet: Option<&str>,
) -> Result<String, String> {
    let query = input
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'query' parameter")?;

    let exact_match = input
        .get("exact_match")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let specific_columns: Option<Vec<&str>> = input
        .get("columns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());

    // Read Excel data
    let options = ExcelReadOptions {
        path: file_path.to_string(),
        sheet: sheet.map(|s| s.to_string()),
        range: None,
        offset: None,
        max_rows: Some(10000),
    };

    let result = excel::read_excel(&options)
        .map_err(|e| format!("Failed to read Excel: {}", e))?;

    // Determine which columns to search
    let search_col_indices: Vec<usize> = if let Some(cols) = specific_columns {
        cols.iter()
            .filter_map(|c| find_column_index(&result.columns, c).ok())
            .collect()
    } else {
        (0..result.columns.len()).collect()
    };

    // Search rows
    let mut found_rows: Vec<(usize, &Vec<CellValue>, Vec<usize>)> = Vec::new();
    let query_lower = query.to_lowercase();

    for (row_idx, row) in result.rows.iter().enumerate() {
        if found_rows.len() >= limit {
            break;
        }

        let mut matching_cols: Vec<usize> = Vec::new();

        for &col_idx in &search_col_indices {
            if let Some(cell) = row.get(col_idx) {
                let cell_str = cell_to_string(cell);
                let cell_lower = cell_str.to_lowercase();

                let matches = if exact_match {
                    cell_lower == query_lower
                } else {
                    cell_lower.contains(&query_lower)
                };

                if matches {
                    matching_cols.push(col_idx);
                }
            }
        }

        if !matching_cols.is_empty() {
            found_rows.push((result.offset as usize + row_idx + 1, row, matching_cols));
        }
    }

    if found_rows.is_empty() {
        return Ok(format!("No matches found for: \"{}\"", query));
    }

    // Format output
    let rows_ref: Vec<(usize, &Vec<CellValue>)> = found_rows.iter().map(|(i, r, _)| (*i, *r)).collect();
    let mut output = format_table_output(&result.columns, &rows_ref, limit)?;

    // Add match info
    output.push_str(&format!("\n\nFound {} matches", found_rows.len()));
    if found_rows.len() > limit {
        output.push_str(&format!(" (showing first {})", limit));
    }

    Ok(output)
}

/// Execute excel_aggregate tool
pub fn execute_aggregate(
    input: &serde_json::Value,
    file_path: &str,
    sheet: Option<&str>,
) -> Result<String, String> {
    let column = input
        .get("column")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'column' parameter")?;

    let operations: Vec<&str> = input
        .get("operations")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_else(|| vec!["sum", "avg", "min", "max", "count"]);

    let group_by = input.get("group_by").and_then(|v| v.as_str());

    // Read Excel data
    let options = ExcelReadOptions {
        path: file_path.to_string(),
        sheet: sheet.map(|s| s.to_string()),
        range: None,
        offset: None,
        max_rows: Some(100000), // Read more for aggregates
    };

    let result = excel::read_excel(&options)
        .map_err(|e| format!("Failed to read Excel: {}", e))?;

    let col_idx = find_column_index(&result.columns, column)?;
    let col_header = result.columns.get(col_idx)
        .map(|c| c.header.clone().unwrap_or(c.name.clone()))
        .unwrap_or(column.to_string());

    if let Some(group_col) = group_by {
        // Grouped aggregation
        let group_idx = find_column_index(&result.columns, group_col)?;
        let group_header = result.columns.get(group_idx)
            .map(|c| c.header.clone().unwrap_or(c.name.clone()))
            .unwrap_or(group_col.to_string());

        let mut groups: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

        for row in &result.rows {
            let group_key = row.get(group_idx)
                .map(|c| cell_to_string(c))
                .unwrap_or_default();

            if let Some(cell) = row.get(col_idx) {
                if let Some(num) = cell_to_number(cell) {
                    groups.entry(group_key).or_insert_with(Vec::new).push(num);
                }
            }
        }

        if groups.is_empty() {
            return Ok(format!("No numeric values found in column '{}'", column));
        }

        // Build output table
        let mut output = format!("## Aggregation of '{}' grouped by '{}'\n\n", col_header, group_header);
        output.push_str("| Group | ");
        output.push_str(&operations.iter().map(|op| format!("{} |", op)).collect::<String>());
        output.push('\n');
        output.push_str("|---|");
        output.push_str(&operations.iter().map(|_| "---|").collect::<String>());
        output.push('\n');

        let mut sorted_groups: Vec<_> = groups.iter().collect();
        sorted_groups.sort_by(|a, b| a.0.cmp(b.0));

        for (group_name, values) in sorted_groups {
            output.push_str(&format!("| {} |", group_name));
            for op in &operations {
                let result = compute_aggregate(values, op);
                output.push_str(&format!(" {} |", format_number(result)));
            }
            output.push('\n');
        }

        Ok(output)
    } else {
        // Simple aggregation
        let values: Vec<f64> = result.rows.iter()
            .filter_map(|row| row.get(col_idx).and_then(cell_to_number))
            .collect();

        if values.is_empty() {
            return Ok(format!("No numeric values found in column '{}'", column));
        }

        let mut output = format!("## Aggregation of '{}'\n\n", col_header);
        output.push_str("| Metric | Value |\n");
        output.push_str("|---|---|\n");

        for op in &operations {
            let result = compute_aggregate(&values, op);
            output.push_str(&format!("| {} | {} |\n", op, format_number(result)));
        }

        output.push_str(&format!("\n*Based on {} numeric values*", values.len()));

        Ok(output)
    }
}

// Helper functions

fn find_column_index(columns: &[excel::ColumnInfo], column: &str) -> Result<usize, String> {
    // Try by name first
    for (i, col) in columns.iter().enumerate() {
        if col.name.eq_ignore_ascii_case(column) {
            return Ok(i);
        }
        if let Some(ref header) = col.header {
            if header.eq_ignore_ascii_case(column) {
                return Ok(i);
            }
        }
    }

    // Try as letter index (A, B, C, etc.)
    if column.len() <= 2 && column.chars().all(|c| c.is_ascii_alphabetic()) {
        let mut idx = 0usize;
        for c in column.to_uppercase().chars() {
            idx = idx * 26 + (c as usize - 'A' as usize + 1);
        }
        if idx > 0 && idx <= columns.len() {
            return Ok(idx - 1);
        }
    }

    // Try as numeric index
    if let Ok(idx) = column.parse::<usize>() {
        if idx < columns.len() {
            return Ok(idx);
        }
    }

    Err(format!("Column '{}' not found. Available columns: {}",
        column,
        columns.iter()
            .map(|c| c.header.clone().unwrap_or(c.name.clone()))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn cell_to_string(cell: &CellValue) -> String {
    match cell {
        CellValue::Number(n) => format!("{}", n),
        CellValue::String(s) => s.clone(),
        CellValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::DateTime(dt) => format!("{}", dt),
        CellValue::Empty => String::new(),
        CellValue::Error(e) => format!("#ERROR: {}", e),
        CellValue::Formula { cached_value, .. } => {
            if let Some(cv) = cached_value {
                cell_to_string(cv)
            } else {
                String::from("#FORMULA")
            }
        }
    }
}

fn cell_to_number(cell: &CellValue) -> Option<f64> {
    match cell {
        CellValue::Number(n) => Some(*n),
        CellValue::String(s) => s.parse().ok(),
        CellValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        CellValue::Formula { cached_value: Some(cv), .. } => cell_to_number(cv),
        _ => None,
    }
}

fn compare_numeric<F>(cell: &CellValue, value: &str, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let cell_num = cell_to_number(cell);
    let value_num: Option<f64> = value.parse().ok();

    match (cell_num, value_num) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => false,
    }
}

fn format_table_output(
    columns: &[excel::ColumnInfo],
    rows: &[(usize, &Vec<CellValue>)],
    limit: usize,
) -> Result<String, String> {
    if rows.is_empty() {
        return Ok("No matching rows found.".to_string());
    }

    let headers: Vec<String> = columns.iter()
        .map(|c| c.header.clone().unwrap_or(c.name.clone()))
        .collect();

    let mut output = String::new();

    // Header row
    output.push_str("| Row | ");
    output.push_str(&headers.join(" | "));
    output.push_str(" |\n");

    // Separator
    output.push_str("|---|");
    output.push_str(&headers.iter().map(|_| "---").collect::<Vec<_>>().join("|"));
    output.push_str("|\n");

    // Data rows
    for (row_num, row) in rows.iter().take(limit) {
        output.push_str(&format!("| {} |", row_num));
        for (col_idx, _) in columns.iter().enumerate() {
            let cell = row.get(col_idx).cloned().unwrap_or(CellValue::Empty);
            let display = cell_to_string(&cell);
            // Truncate long values
            let truncated = if display.len() > 30 {
                format!("{}...", &display[..27])
            } else {
                display
            };
            output.push_str(&format!(" {} |", truncated));
        }
        output.push('\n');
    }

    if rows.len() > limit {
        output.push_str(&format!("\n*Showing {} of {} matching rows*", limit, rows.len()));
    }

    Ok(output)
}

fn compute_aggregate(values: &[f64], operation: &str) -> f64 {
    match operation {
        "sum" => values.iter().sum(),
        "avg" => {
            if values.is_empty() { 0.0 }
            else { values.iter().sum::<f64>() / values.len() as f64 }
        }
        "min" => values.iter().cloned().fold(f64::INFINITY, f64::min),
        "max" => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "count" => values.len() as f64,
        "count_distinct" => {
            let mut unique: Vec<i64> = values.iter().map(|v| (*v * 1000000.0) as i64).collect();
            unique.sort();
            unique.dedup();
            unique.len() as f64
        }
        "median" => {
            if values.is_empty() { return 0.0; }
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 0 {
                (sorted[mid - 1] + sorted[mid]) / 2.0
            } else {
                sorted[mid]
            }
        }
        _ => 0.0,
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{:.0}", n)
    } else if n.abs() >= 1000.0 || n.abs() < 0.01 {
        format!("{:.2e}", n)
    } else {
        format!("{:.2}", n)
    }
}
