use regex::Regex;
use std::collections::HashMap;

use super::types::*;

/// Validate data against a schema
pub fn validate_schema(
    rows: &[Vec<CellValue>],
    columns: &[ColumnInfo],
    schema: &ExcelSchema,
    offset: u32,
) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Build column mapping: schema column name -> actual column index
    let column_map = build_column_map(columns, schema);

    // Check for missing required columns
    for col_schema in &schema.columns {
        if col_schema.required && !column_map.contains_key(&col_schema.name) {
            errors.push(ValidationError {
                row: 0,
                col: 0,
                column_name: col_schema.name.clone(),
                message: format!("Required column '{}' not found", col_schema.name),
                error_type: ValidationErrorType::MissingColumn,
                current_value: CellValue::Empty,
            });
        }
    }

    // Determine start row (skip header if present)
    let start_row = if schema.has_header_row && offset == 0 { 1 } else { 0 };

    // Validate each row
    for (row_idx, row) in rows.iter().enumerate().skip(start_row) {
        let actual_row = offset + row_idx as u32;

        for col_schema in &schema.columns {
            if let Some(&col_idx) = column_map.get(&col_schema.name) {
                let cell_value = row.get(col_idx).cloned().unwrap_or(CellValue::Empty);

                if let Some(error) = validate_cell(&cell_value, col_schema, actual_row, col_idx as u32) {
                    errors.push(error);
                }

                // Check for warnings
                if let Some(warning) = check_warnings(&cell_value, col_schema, actual_row, col_idx as u32) {
                    warnings.push(warning);
                }
            }
        }
    }

    let rows_checked = rows.len().saturating_sub(start_row) as u32;

    ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
        rows_checked,
    }
}

/// Build a mapping from schema column names to actual column indices
fn build_column_map(columns: &[ColumnInfo], schema: &ExcelSchema) -> HashMap<String, usize> {
    let mut map = HashMap::new();

    for col_schema in &schema.columns {
        let name = &col_schema.name;

        // Try to match by header first
        if let Some(idx) = columns.iter().position(|c| {
            c.header.as_ref().map(|h| h.eq_ignore_ascii_case(name)).unwrap_or(false)
        }) {
            map.insert(name.clone(), idx);
            continue;
        }

        // Try to match by column letter (A, B, C, etc.)
        if let Some(idx) = columns.iter().position(|c| c.name.eq_ignore_ascii_case(name)) {
            map.insert(name.clone(), idx);
            continue;
        }

        // Try to match by index if name is a number
        if let Ok(idx) = name.parse::<usize>() {
            if idx < columns.len() {
                map.insert(name.clone(), idx);
            }
        }
    }

    map
}

/// Validate a single cell against its column schema
fn validate_cell(
    value: &CellValue,
    schema: &ColumnSchema,
    row: u32,
    col: u32,
) -> Option<ValidationError> {
    // Check for empty values
    let is_empty = matches!(value, CellValue::Empty);

    if is_empty {
        if schema.required && !schema.allow_empty {
            return Some(ValidationError {
                row,
                col,
                column_name: schema.name.clone(),
                message: format!("Required field '{}' is empty", schema.name),
                error_type: ValidationErrorType::MissingRequired,
                current_value: value.clone(),
            });
        }
        // Empty but allowed
        return None;
    }

    // Type validation
    if let Some(error) = validate_type(value, schema, row, col) {
        return Some(error);
    }

    // Range validation for numbers
    if let CellValue::Number(n) = value {
        if let Some(min) = schema.min_value {
            if *n < min {
                return Some(ValidationError {
                    row,
                    col,
                    column_name: schema.name.clone(),
                    message: format!("Value {} is below minimum {}", n, min),
                    error_type: ValidationErrorType::OutOfRange,
                    current_value: value.clone(),
                });
            }
        }
        if let Some(max) = schema.max_value {
            if *n > max {
                return Some(ValidationError {
                    row,
                    col,
                    column_name: schema.name.clone(),
                    message: format!("Value {} is above maximum {}", n, max),
                    error_type: ValidationErrorType::OutOfRange,
                    current_value: value.clone(),
                });
            }
        }
    }

    // Regex pattern validation for strings
    if let CellValue::String(s) = value {
        if let Some(ref pattern) = schema.regex_pattern {
            if let Ok(regex) = Regex::new(pattern) {
                if !regex.is_match(s) {
                    return Some(ValidationError {
                        row,
                        col,
                        column_name: schema.name.clone(),
                        message: format!("Value '{}' does not match pattern '{}'", s, pattern),
                        error_type: ValidationErrorType::PatternMismatch,
                        current_value: value.clone(),
                    });
                }
            }
        }

        // Allowed values validation
        if let Some(ref allowed) = schema.allowed_values {
            if !allowed.iter().any(|v| v.eq_ignore_ascii_case(s)) {
                return Some(ValidationError {
                    row,
                    col,
                    column_name: schema.name.clone(),
                    message: format!(
                        "Value '{}' is not in allowed values: {:?}",
                        s, allowed
                    ),
                    error_type: ValidationErrorType::InvalidValue,
                    current_value: value.clone(),
                });
            }
        }
    }

    None
}

/// Validate that the cell value matches the expected type
fn validate_type(
    value: &CellValue,
    schema: &ColumnSchema,
    row: u32,
    col: u32,
) -> Option<ValidationError> {
    let expected_type = &schema.data_type;

    if expected_type == "any" {
        return None;
    }

    let type_matches = match (value, expected_type.as_str()) {
        (CellValue::String(_), "string") => true,
        (CellValue::Number(_), "number") => true,
        (CellValue::Boolean(_), "boolean") => true,
        (CellValue::DateTime(_), "date") => true,
        (CellValue::DateTime(_), "datetime") => true,
        // Allow numbers to be treated as strings
        (CellValue::Number(_), "string") => true,
        // Allow booleans to be treated as strings
        (CellValue::Boolean(_), "string") => true,
        // Allow strings that look like numbers to match number type
        (CellValue::String(s), "number") => s.parse::<f64>().is_ok(),
        // Allow strings that look like booleans
        (CellValue::String(s), "boolean") => {
            matches!(s.to_lowercase().as_str(), "true" | "false" | "yes" | "no" | "1" | "0")
        }
        // Formulas - check cached value type
        (CellValue::Formula { cached_value: Some(cv), .. }, expected) => {
            validate_type(cv, schema, row, col).is_none()
        }
        _ => false,
    };

    if !type_matches {
        let actual_type = match value {
            CellValue::Empty => "empty",
            CellValue::String(_) => "string",
            CellValue::Number(_) => "number",
            CellValue::Boolean(_) => "boolean",
            CellValue::DateTime(_) => "datetime",
            CellValue::Error(_) => "error",
            CellValue::Formula { .. } => "formula",
        };

        return Some(ValidationError {
            row,
            col,
            column_name: schema.name.clone(),
            message: format!(
                "Expected type '{}' but got '{}'",
                expected_type, actual_type
            ),
            error_type: ValidationErrorType::TypeMismatch,
            current_value: value.clone(),
        });
    }

    None
}

/// Check for conditions that warrant warnings (not errors)
fn check_warnings(
    value: &CellValue,
    schema: &ColumnSchema,
    row: u32,
    col: u32,
) -> Option<ValidationWarning> {
    // Warn about very long strings
    if let CellValue::String(s) = value {
        if s.len() > 1000 {
            return Some(ValidationWarning {
                row,
                col,
                column_name: schema.name.clone(),
                message: format!("Very long string value ({} characters)", s.len()),
            });
        }
    }

    // Warn about unusual numbers
    if let CellValue::Number(n) = value {
        if n.is_nan() {
            return Some(ValidationWarning {
                row,
                col,
                column_name: schema.name.clone(),
                message: "Value is NaN".to_string(),
            });
        }
        if n.is_infinite() {
            return Some(ValidationWarning {
                row,
                col,
                column_name: schema.name.clone(),
                message: "Value is infinite".to_string(),
            });
        }
    }

    // Warn about errors
    if let CellValue::Error(e) = value {
        return Some(ValidationWarning {
            row,
            col,
            column_name: schema.name.clone(),
            message: format!("Cell contains error: {}", e),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_required_field() {
        let schema = ExcelSchema {
            columns: vec![ColumnSchema {
                name: "Name".to_string(),
                data_type: "string".to_string(),
                required: true,
                allow_empty: false,
                min_value: None,
                max_value: None,
                regex_pattern: None,
                allowed_values: None,
            }],
            has_header_row: true,
        };

        let columns = vec![ColumnInfo {
            index: 0,
            name: "A".to_string(),
            header: Some("Name".to_string()),
            width: None,
            data_type: Some("string".to_string()),
        }];

        // Header row + empty data row
        let rows = vec![
            vec![CellValue::String("Name".to_string())],
            vec![CellValue::Empty],
        ];

        let result = validate_schema(&rows, &columns, &schema, 0);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(result.errors[0].error_type, ValidationErrorType::MissingRequired));
    }

    #[test]
    fn test_validate_number_range() {
        let schema = ExcelSchema {
            columns: vec![ColumnSchema {
                name: "Age".to_string(),
                data_type: "number".to_string(),
                required: true,
                allow_empty: false,
                min_value: Some(0.0),
                max_value: Some(150.0),
                regex_pattern: None,
                allowed_values: None,
            }],
            has_header_row: true,
        };

        let columns = vec![ColumnInfo {
            index: 0,
            name: "A".to_string(),
            header: Some("Age".to_string()),
            width: None,
            data_type: Some("number".to_string()),
        }];

        let rows = vec![
            vec![CellValue::String("Age".to_string())],
            vec![CellValue::Number(-5.0)],
        ];

        let result = validate_schema(&rows, &columns, &schema, 0);
        assert!(!result.is_valid);
        assert!(matches!(result.errors[0].error_type, ValidationErrorType::OutOfRange));
    }
}
