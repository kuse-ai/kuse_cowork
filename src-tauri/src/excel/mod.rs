//! Excel module for reading, validating, writing, and watching Excel files.
//!
//! This module provides:
//! - Reading Excel files with pagination support
//! - Schema-based validation
//! - Writing edits while preserving formulas and formatting
//! - File change detection

pub mod types;
pub mod reader;
pub mod validator;
pub mod writer;
pub mod watcher;

// Re-export commonly used types and functions
pub use types::*;
pub use reader::{read_excel, get_sheets, compute_checksum};
pub use validator::validate_schema;
pub use writer::{apply_edits, create_backup, export_to_new_file};
pub use watcher::{ExcelWatcher, create_event_channel};
