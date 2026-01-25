use serde::{Deserialize, Serialize};

/// Information about a sheet in an Excel file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetInfo {
    pub name: String,
    pub index: u32,
    pub row_count: u32,
    pub col_count: u32,
}

/// Information about a named range in an Excel file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedRange {
    pub name: String,
    pub sheet: String,
    pub range: String,
}

/// Represents a cell value with type information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CellValue {
    Empty,
    String(String),
    Number(f64),
    Boolean(bool),
    DateTime(String), // ISO 8601 format
    Error(String),
    Formula { formula: String, cached_value: Option<Box<CellValue>> },
}

impl Default for CellValue {
    fn default() -> Self {
        CellValue::Empty
    }
}

/// Column metadata for the data grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub index: u32,
    pub name: String,      // Column letter (A, B, C, etc.)
    pub header: Option<String>, // First row value if it's a header
    pub width: Option<f64>,
    pub data_type: Option<String>, // Inferred type: "string", "number", "date", "boolean", "mixed"
}

/// Size information for the file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeInfo {
    pub file_size_bytes: u64,
    pub row_count: u32,
    pub col_count: u32,
}

/// Result of reading an Excel file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelReadResult {
    pub sheets: Vec<SheetInfo>,
    pub named_ranges: Vec<NamedRange>,
    pub rows: Vec<Vec<CellValue>>,
    pub columns: Vec<ColumnInfo>,
    pub checksum: String,
    pub size_info: SizeInfo,
    pub total_rows: u32,
    pub has_more: bool,
    pub offset: u32,
}

/// Schema definition for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelSchema {
    pub columns: Vec<ColumnSchema>,
    pub has_header_row: bool,
}

/// Schema for a single column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,           // Expected column header or letter
    pub data_type: String,      // "string", "number", "date", "boolean", "any"
    pub required: bool,
    pub allow_empty: bool,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub regex_pattern: Option<String>,
    pub allowed_values: Option<Vec<String>>,
}

/// A single validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub row: u32,
    pub col: u32,
    pub column_name: String,
    pub message: String,
    pub error_type: ValidationErrorType,
    pub current_value: CellValue,
}

/// Types of validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    MissingRequired,
    TypeMismatch,
    OutOfRange,
    PatternMismatch,
    InvalidValue,
    MissingColumn,
}

/// A validation warning (less severe than error)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub row: u32,
    pub col: u32,
    pub column_name: String,
    pub message: String,
}

/// Result of schema validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub rows_checked: u32,
}

/// A cell edit to be applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellEdit {
    pub row: u32,
    pub col: u32,
    pub value: CellValue,
    pub original_value: Option<CellValue>,
}

/// Result of applying edits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub success: bool,
    pub edits_applied: u32,
    pub new_checksum: String,
    pub errors: Vec<String>,
}

/// Excel-specific errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelError {
    pub message: String,
    pub error_type: ExcelErrorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExcelErrorType {
    FileNotFound,
    InvalidFormat,
    SheetNotFound,
    ReadError,
    WriteError,
    WatchError,
    PermissionDenied,
    FileLocked,
    ChecksumMismatch,
}

impl std::fmt::Display for ExcelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExcelError {}

impl ExcelError {
    pub fn new(message: impl Into<String>, error_type: ExcelErrorType) -> Self {
        ExcelError {
            message: message.into(),
            error_type,
        }
    }

    pub fn file_not_found(path: &str) -> Self {
        ExcelError::new(format!("File not found: {}", path), ExcelErrorType::FileNotFound)
    }

    pub fn invalid_format(message: impl Into<String>) -> Self {
        ExcelError::new(message, ExcelErrorType::InvalidFormat)
    }

    pub fn sheet_not_found(sheet: &str) -> Self {
        ExcelError::new(format!("Sheet not found: {}", sheet), ExcelErrorType::SheetNotFound)
    }

    pub fn read_error(message: impl Into<String>) -> Self {
        ExcelError::new(message, ExcelErrorType::ReadError)
    }

    pub fn write_error(message: impl Into<String>) -> Self {
        ExcelError::new(message, ExcelErrorType::WriteError)
    }
}

/// File change event from watcher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeEvent {
    pub path: String,
    pub change_type: FileChangeType,
    pub new_checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChangeType {
    Modified,
    Deleted,
    Renamed,
}

/// Options for reading Excel files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcelReadOptions {
    pub path: String,
    pub sheet: Option<String>,
    pub range: Option<String>,
    pub offset: Option<u32>,
    pub max_rows: Option<u32>,
}
