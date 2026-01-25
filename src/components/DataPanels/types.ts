// Re-export types from excel-api for convenience
export type {
  SheetInfo,
  NamedRange,
  CellValue,
  ColumnInfo,
  SizeInfo,
  ExcelReadResult,
  ColumnSchema,
  ExcelSchema,
  ValidationErrorType,
  ValidationError,
  ValidationWarning,
  ValidationResult,
  CellEdit,
  ApplyResult,
  FileChangeEvent,
  DataPanel,
} from "../../lib/excel-api";

// Component-specific types
export interface DataGridProps {
  rows: import("../../lib/excel-api").CellValue[][];
  columns: import("../../lib/excel-api").ColumnInfo[];
  validationErrors: import("../../lib/excel-api").ValidationError[];
  pendingEdits: Map<string, import("../../lib/excel-api").CellEdit>;
  offset: number;
  totalRows: number;
  isLoading: boolean;
  hasMore: boolean;
  onCellEdit: (row: number, col: number, value: string | number | boolean | null) => void;
  onLoadMore: () => void;
}

export interface FileSelectorProps {
  filePath: string | null;
  recentFiles: string[];
  lastSyncTime: number | null;
  hasFileChanged: boolean;
  onOpenFile: (path?: string) => Promise<boolean>;
  onRefresh: () => Promise<void>;
}

export interface SheetSelectorProps {
  sheets: import("../../lib/excel-api").SheetInfo[];
  activeSheet: string | null;
  activeRange: string | null;
  onSheetSelect: (sheet: string) => void;
  onRangeChange: (range: string | null) => void;
}

export interface ActionBarProps {
  pendingEditCount: number;
  isLoading: boolean;
  watchEnabled: boolean;
  hasSchema: boolean;
  canApply: boolean;
  onRefresh: () => void;
  onValidate: () => void;
  onApply: () => void;
  onDownload: () => void;
  onOpenExternal: () => void;
  onToggleWatch: () => void;
}

export interface ValidationBadgeProps {
  type: "error" | "warning";
  message: string;
}

export interface EditDialogProps {
  isOpen: boolean;
  pendingEdits: Map<string, import("../../lib/excel-api").CellEdit>;
  onConfirm: () => void;
  onCancel: () => void;
  onRemoveEdit: (row: number, col: number) => void;
}

export type DataProvider = "excel" | "sheets" | "csv";
