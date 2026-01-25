import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { isTauri } from "./tauri-api";

// ==================== Types ====================

export interface SheetInfo {
  name: string;
  index: number;
  row_count: number;
  col_count: number;
}

export interface NamedRange {
  name: string;
  sheet: string;
  range: string;
}

export type CellValue =
  | { type: "Empty" }
  | { type: "String"; value: string }
  | { type: "Number"; value: number }
  | { type: "Boolean"; value: boolean }
  | { type: "DateTime"; value: string }
  | { type: "Error"; value: string }
  | { type: "Formula"; value: { formula: string; cached_value: CellValue | null } };

export interface ColumnInfo {
  index: number;
  name: string;       // Column letter (A, B, C, etc.)
  header: string | null;
  width: number | null;
  data_type: string | null;
}

export interface SizeInfo {
  file_size_bytes: number;
  row_count: number;
  col_count: number;
}

export interface ExcelReadResult {
  sheets: SheetInfo[];
  named_ranges: NamedRange[];
  rows: CellValue[][];
  columns: ColumnInfo[];
  checksum: string;
  size_info: SizeInfo;
  total_rows: number;
  has_more: boolean;
  offset: number;
}

export interface ColumnSchema {
  name: string;
  data_type: string;  // "string", "number", "date", "boolean", "any"
  required: boolean;
  allow_empty: boolean;
  min_value?: number;
  max_value?: number;
  regex_pattern?: string;
  allowed_values?: string[];
}

export interface ExcelSchema {
  columns: ColumnSchema[];
  has_header_row: boolean;
}

export type ValidationErrorType =
  | "MissingRequired"
  | "TypeMismatch"
  | "OutOfRange"
  | "PatternMismatch"
  | "InvalidValue"
  | "MissingColumn";

export interface ValidationError {
  row: number;
  col: number;
  column_name: string;
  message: string;
  error_type: ValidationErrorType;
  current_value: CellValue;
}

export interface ValidationWarning {
  row: number;
  col: number;
  column_name: string;
  message: string;
}

export interface ValidationResult {
  is_valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
  rows_checked: number;
}

export interface CellEdit {
  row: number;
  col: number;
  value: CellValue;
  original_value?: CellValue;
}

export interface ApplyResult {
  success: boolean;
  edits_applied: number;
  new_checksum: string;
  errors: string[];
}

export interface FileChangeEvent {
  path: string;
  change_type: "Modified" | "Deleted" | "Renamed";
  new_checksum: string | null;
}

export interface DataPanel {
  id: string;
  provider: string;
  config: string;
  created_at: number;
  updated_at: number;
}

// ==================== Excel API ====================

export interface ExcelReadOptions {
  path: string;
  sheet?: string;
  range?: string;
  offset?: number;
  maxRows?: number;
}

export async function excelRead(options: ExcelReadOptions): Promise<ExcelReadResult> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<ExcelReadResult>("excel_read", {
    path: options.path,
    sheet: options.sheet,
    range: options.range,
    offset: options.offset,
    maxRows: options.maxRows,
  });
}

export interface ExcelValidateOptions {
  path: string;
  sheet?: string;
  range?: string;
  schema: ExcelSchema;
  offset?: number;
  maxRows?: number;
}

export async function excelValidate(options: ExcelValidateOptions): Promise<ValidationResult> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<ValidationResult>("excel_validate", {
    path: options.path,
    sheet: options.sheet,
    range: options.range,
    schema: options.schema,
    offset: options.offset,
    maxRows: options.maxRows,
  });
}

export interface ExcelApplyOptions {
  path: string;
  sheet: string;
  edits: CellEdit[];
  validateChecksum?: string;
}

export async function excelApply(options: ExcelApplyOptions): Promise<ApplyResult> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<ApplyResult>("excel_apply", {
    path: options.path,
    sheet: options.sheet,
    edits: options.edits,
    validateChecksum: options.validateChecksum,
  });
}

export async function excelWatch(path: string, enable: boolean): Promise<void> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<void>("excel_watch", { path, enable });
}

export async function excelGetSheets(path: string): Promise<SheetInfo[]> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<SheetInfo[]>("excel_get_sheets", { path });
}

export async function excelChecksum(path: string): Promise<string> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<string>("excel_checksum", { path });
}

export async function excelBackup(path: string): Promise<string> {
  if (!isTauri()) {
    throw new Error("Excel operations require the Tauri app");
  }

  return invoke<string>("excel_backup", { path });
}

// ==================== Event Listeners ====================

export function listenToFileChange(
  callback: (event: FileChangeEvent) => void
): Promise<UnlistenFn> {
  return listen<FileChangeEvent>("excel-file-changed", (event) => {
    callback(event.payload);
  });
}

export function listenToEditsApplied(
  callback: (event: { path: string; sheet: string; edits_applied: number; new_checksum: string }) => void
): Promise<UnlistenFn> {
  return listen("excel-edits-applied", (event) => {
    callback(event.payload as any);
  });
}

// ==================== Data Panel API ====================

export async function getDataPanel(provider: string): Promise<DataPanel | null> {
  if (!isTauri()) {
    // Fallback for web dev
    const stored = localStorage.getItem(`kuse-data-panel-${provider}`);
    if (stored) {
      return JSON.parse(stored);
    }
    return null;
  }

  return invoke<DataPanel | null>("get_data_panel", { provider });
}

export async function saveDataPanel(provider: string, config: object): Promise<DataPanel> {
  if (!isTauri()) {
    // Fallback for web dev
    const panel: DataPanel = {
      id: crypto.randomUUID(),
      provider,
      config: JSON.stringify(config),
      created_at: Date.now(),
      updated_at: Date.now(),
    };
    localStorage.setItem(`kuse-data-panel-${provider}`, JSON.stringify(panel));
    return panel;
  }

  return invoke<DataPanel>("save_data_panel", {
    provider,
    config: JSON.stringify(config),
  });
}

export async function deleteDataPanel(id: string): Promise<void> {
  if (!isTauri()) {
    // Fallback for web dev - we'd need to track by id
    return;
  }

  return invoke<void>("delete_data_panel", { id });
}

export async function listDataPanels(): Promise<DataPanel[]> {
  if (!isTauri()) {
    return [];
  }

  return invoke<DataPanel[]>("list_data_panels");
}

// ==================== Helper Functions ====================

export function getCellDisplayValue(cell: CellValue): string {
  switch (cell.type) {
    case "Empty":
      return "";
    case "String":
      return cell.value;
    case "Number":
      return cell.value.toString();
    case "Boolean":
      return cell.value ? "TRUE" : "FALSE";
    case "DateTime":
      return cell.value;
    case "Error":
      return `#${cell.value}`;
    case "Formula":
      // Display cached value if available, otherwise show formula
      if (cell.value.cached_value) {
        return getCellDisplayValue(cell.value.cached_value);
      }
      return `=${cell.value.formula}`;
    default:
      return "";
  }
}

export function createCellValue(value: string | number | boolean | null): CellValue {
  if (value === null || value === "") {
    return { type: "Empty" };
  }
  if (typeof value === "number") {
    return { type: "Number", value };
  }
  if (typeof value === "boolean") {
    return { type: "Boolean", value };
  }
  // Try to parse as number
  const num = parseFloat(value);
  if (!isNaN(num) && isFinite(num)) {
    return { type: "Number", value: num };
  }
  return { type: "String", value };
}

export function columnLetterToIndex(letter: string): number {
  let result = 0;
  for (let i = 0; i < letter.length; i++) {
    result = result * 26 + letter.charCodeAt(i) - 'A'.charCodeAt(0) + 1;
  }
  return result - 1;
}

export function columnIndexToLetter(index: number): string {
  let result = "";
  let n = index + 1;
  while (n > 0) {
    n--;
    result = String.fromCharCode('A'.charCodeAt(0) + (n % 26)) + result;
    n = Math.floor(n / 26);
  }
  return result;
}
