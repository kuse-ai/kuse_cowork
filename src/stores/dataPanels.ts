import { createSignal, createEffect, onCleanup } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import {
  excelRead,
  excelValidate,
  excelApply,
  excelWatch,
  excelChecksum,
  getDataPanel,
  saveDataPanel,
  listenToFileChange,
  listenToEditsApplied,
  CellValue,
  CellEdit,
  ColumnInfo,
  SheetInfo,
  ExcelSchema,
  ValidationError,
  ValidationWarning,
  createCellValue,
  getCellDisplayValue,
  FileChangeEvent,
} from "../lib/excel-api";

// ==================== Types ====================

export interface ExcelPanelConfig {
  filePath: string | null;
  activeSheet: string | null;
  activeRange: string | null;
  schema: ExcelSchema | null;
  recentFiles: string[];
  watchEnabled: boolean;
}

export interface ExcelPanelState {
  filePath: string | null;
  sheets: SheetInfo[];
  activeSheet: string | null;
  activeRange: string | null;
  rows: CellValue[][];
  columns: ColumnInfo[];
  checksum: string | null;
  totalRows: number;
  offset: number;
  schema: ExcelSchema | null;
  validationErrors: ValidationError[];
  validationWarnings: ValidationWarning[];
  pendingEdits: Map<string, CellEdit>;
  recentFiles: string[];
  watchEnabled: boolean;
  isLoading: boolean;
  lastSyncTime: number | null;
  hasFileChanged: boolean;
  error: string | null;
}

// ==================== Signals ====================

const [showDataPanels, setShowDataPanels] = createSignal(false);
const [activeProvider, setActiveProvider] = createSignal<"excel" | "sheets" | "csv">("excel");

const defaultExcelState: ExcelPanelState = {
  filePath: null,
  sheets: [],
  activeSheet: null,
  activeRange: null,
  rows: [],
  columns: [],
  checksum: null,
  totalRows: 0,
  offset: 0,
  schema: null,
  validationErrors: [],
  validationWarnings: [],
  pendingEdits: new Map(),
  recentFiles: [],
  watchEnabled: false,
  isLoading: false,
  lastSyncTime: null,
  hasFileChanged: false,
  error: null,
};

const [excelState, setExcelState] = createSignal<ExcelPanelState>({ ...defaultExcelState });

// ==================== Actions ====================

async function openExcelFile(path?: string): Promise<boolean> {
  let filePath = path;

  if (!filePath) {
    // Open file dialog
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Excel Files",
          extensions: ["xlsx", "xls", "xlsm", "xlsb"],
        },
        {
          name: "All Files",
          extensions: ["*"],
        },
      ],
    });

    if (!selected || typeof selected !== "string") {
      return false;
    }

    filePath = selected;
  }

  setExcelState((prev) => ({
    ...prev,
    isLoading: true,
    error: null,
  }));

  try {
    const result = await excelRead({
      path: filePath,
      maxRows: 1000,
    });

    // Update recent files
    const recentFiles = [
      filePath,
      ...excelState().recentFiles.filter((f) => f !== filePath),
    ].slice(0, 10);

    setExcelState((prev) => ({
      ...prev,
      filePath,
      sheets: result.sheets,
      activeSheet: result.sheets[0]?.name || null,
      rows: result.rows,
      columns: result.columns,
      checksum: result.checksum,
      totalRows: result.total_rows,
      offset: result.offset,
      isLoading: false,
      lastSyncTime: Date.now(),
      hasFileChanged: false,
      recentFiles,
      pendingEdits: new Map(),
      validationErrors: [],
      validationWarnings: [],
    }));

    // Start watching if enabled
    if (excelState().watchEnabled) {
      await excelWatch(filePath, true);
    }

    // Save state
    await savePanelState();

    return true;
  } catch (error) {
    setExcelState((prev) => ({
      ...prev,
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    }));
    return false;
  }
}

async function selectSheet(sheet: string): Promise<void> {
  const state = excelState();
  if (!state.filePath) return;

  setExcelState((prev) => ({ ...prev, isLoading: true, error: null }));

  try {
    const result = await excelRead({
      path: state.filePath,
      sheet,
      offset: 0,
      maxRows: 1000,
    });

    setExcelState((prev) => ({
      ...prev,
      activeSheet: sheet,
      rows: result.rows,
      columns: result.columns,
      totalRows: result.total_rows,
      offset: 0,
      isLoading: false,
      pendingEdits: new Map(),
      validationErrors: [],
      validationWarnings: [],
    }));

    await savePanelState();
  } catch (error) {
    setExcelState((prev) => ({
      ...prev,
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    }));
  }
}

async function loadMoreRows(): Promise<void> {
  const state = excelState();
  if (!state.filePath || !state.activeSheet) return;
  if (state.offset + state.rows.length >= state.totalRows) return;

  setExcelState((prev) => ({ ...prev, isLoading: true }));

  try {
    const result = await excelRead({
      path: state.filePath,
      sheet: state.activeSheet,
      offset: state.offset + state.rows.length,
      maxRows: 1000,
    });

    setExcelState((prev) => ({
      ...prev,
      rows: [...prev.rows, ...result.rows],
      offset: prev.offset,
      isLoading: false,
    }));
  } catch (error) {
    setExcelState((prev) => ({
      ...prev,
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    }));
  }
}

async function refreshData(): Promise<void> {
  const state = excelState();
  if (!state.filePath) return;

  setExcelState((prev) => ({ ...prev, isLoading: true, error: null }));

  try {
    const result = await excelRead({
      path: state.filePath,
      sheet: state.activeSheet || undefined,
      offset: 0,
      maxRows: 1000,
    });

    setExcelState((prev) => ({
      ...prev,
      rows: result.rows,
      columns: result.columns,
      checksum: result.checksum,
      totalRows: result.total_rows,
      offset: 0,
      isLoading: false,
      lastSyncTime: Date.now(),
      hasFileChanged: false,
      pendingEdits: new Map(),
    }));
  } catch (error) {
    setExcelState((prev) => ({
      ...prev,
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    }));
  }
}

async function validateData(): Promise<ValidationError[]> {
  const state = excelState();
  if (!state.filePath || !state.schema) {
    return [];
  }

  setExcelState((prev) => ({ ...prev, isLoading: true }));

  try {
    const result = await excelValidate({
      path: state.filePath,
      sheet: state.activeSheet || undefined,
      schema: state.schema,
    });

    setExcelState((prev) => ({
      ...prev,
      validationErrors: result.errors,
      validationWarnings: result.warnings,
      isLoading: false,
    }));

    return result.errors;
  } catch (error) {
    setExcelState((prev) => ({
      ...prev,
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    }));
    return [];
  }
}

function addPendingEdit(row: number, col: number, value: string | number | boolean | null): void {
  const state = excelState();
  const key = `${row},${col}`;
  const originalValue = state.rows[row - state.offset]?.[col] || { type: "Empty" as const };

  const newEdits = new Map(state.pendingEdits);
  newEdits.set(key, {
    row,
    col,
    value: createCellValue(value),
    original_value: originalValue,
  });

  setExcelState((prev) => ({
    ...prev,
    pendingEdits: newEdits,
  }));
}

function removePendingEdit(row: number, col: number): void {
  const key = `${row},${col}`;
  const newEdits = new Map(excelState().pendingEdits);
  newEdits.delete(key);

  setExcelState((prev) => ({
    ...prev,
    pendingEdits: newEdits,
  }));
}

function clearPendingEdits(): void {
  setExcelState((prev) => ({
    ...prev,
    pendingEdits: new Map(),
  }));
}

async function applyEdits(): Promise<boolean> {
  const state = excelState();
  if (!state.filePath || !state.activeSheet || state.pendingEdits.size === 0) {
    return false;
  }

  setExcelState((prev) => ({ ...prev, isLoading: true }));

  try {
    const edits = Array.from(state.pendingEdits.values());
    const result = await excelApply({
      path: state.filePath,
      sheet: state.activeSheet,
      edits,
      validateChecksum: state.checksum || undefined,
    });

    if (result.success) {
      // Refresh data to see the changes
      await refreshData();

      setExcelState((prev) => ({
        ...prev,
        checksum: result.new_checksum,
        pendingEdits: new Map(),
        isLoading: false,
      }));

      return true;
    } else {
      setExcelState((prev) => ({
        ...prev,
        isLoading: false,
        error: result.errors.join(", "),
      }));
      return false;
    }
  } catch (error) {
    setExcelState((prev) => ({
      ...prev,
      isLoading: false,
      error: error instanceof Error ? error.message : String(error),
    }));
    return false;
  }
}

async function toggleWatch(): Promise<void> {
  const state = excelState();
  if (!state.filePath) return;

  const newWatchEnabled = !state.watchEnabled;

  try {
    await excelWatch(state.filePath, newWatchEnabled);

    setExcelState((prev) => ({
      ...prev,
      watchEnabled: newWatchEnabled,
    }));

    await savePanelState();
  } catch (error) {
    console.error("Failed to toggle watch:", error);
  }
}

function setSchema(schema: ExcelSchema | null): void {
  setExcelState((prev) => ({
    ...prev,
    schema,
    validationErrors: [],
    validationWarnings: [],
  }));
}

// ==================== Persistence ====================

async function loadPanelState(): Promise<void> {
  try {
    const panel = await getDataPanel("excel");
    if (panel) {
      const config: ExcelPanelConfig = JSON.parse(panel.config);

      setExcelState((prev) => ({
        ...prev,
        recentFiles: config.recentFiles || [],
        schema: config.schema || null,
        watchEnabled: config.watchEnabled || false,
      }));

      // Restore last opened file if available
      if (config.filePath) {
        await openExcelFile(config.filePath);
      }
    }
  } catch (error) {
    console.error("Failed to load panel state:", error);
  }
}

async function savePanelState(): Promise<void> {
  const state = excelState();

  const config: ExcelPanelConfig = {
    filePath: state.filePath,
    activeSheet: state.activeSheet,
    activeRange: state.activeRange,
    schema: state.schema,
    recentFiles: state.recentFiles,
    watchEnabled: state.watchEnabled,
  };

  try {
    await saveDataPanel("excel", config);
  } catch (error) {
    console.error("Failed to save panel state:", error);
  }
}

// ==================== LLM Context Generation ====================

const MAX_ROWS_FULL_INJECT = 100;
const SAMPLE_ROWS_COUNT = 10;

/**
 * Generate a markdown summary of the current Excel data for LLM context injection
 */
function generateExcelContext(): string | null {
  const state = excelState();

  if (!state.filePath || state.rows.length === 0) {
    return null;
  }

  const fileName = state.filePath.split("/").pop() || state.filePath;
  const lines: string[] = [];

  // Header with full file path for tools
  lines.push("## [Excel Data Context]");
  lines.push(`**File**: ${fileName}`);
  lines.push(`**Full Path**: \`${state.filePath}\``);
  lines.push(`**Sheet**: ${state.activeSheet || "Unknown"} (${state.totalRows} rows × ${state.columns.length} columns)`);
  lines.push("");

  // Column info
  const colHeaders = state.columns.map((c) => {
    const header = c.header || c.name;
    return `${c.name}(${header})`;
  }).join(", ");
  lines.push(`**Columns**: ${colHeaders}`);
  lines.push("");

  // Decide whether to inject all data or just a sample
  const injectAll = state.totalRows <= MAX_ROWS_FULL_INJECT;

  if (injectAll) {
    lines.push("**Data** (all rows):");
  } else {
    lines.push(`**Data Preview** (first ${SAMPLE_ROWS_COUNT} of ${state.totalRows} rows):`);
  }

  // Build markdown table
  const headers = state.columns.map((c) => c.header || c.name);
  const headerRow = "| # | " + headers.join(" | ") + " |";
  const separatorRow = "|---|" + headers.map(() => "---").join("|") + "|";

  lines.push(headerRow);
  lines.push(separatorRow);

  const rowsToShow = injectAll ? state.rows : state.rows.slice(0, SAMPLE_ROWS_COUNT);

  for (let i = 0; i < rowsToShow.length; i++) {
    const row = rowsToShow[i];
    const rowNum = state.offset + i + 1;
    const values = row.map((cell) => {
      const display = getCellDisplayValue(cell);
      // Truncate long values
      return display.length > 30 ? display.substring(0, 27) + "..." : display;
    });
    lines.push(`| ${rowNum} | ${values.join(" | ")} |`);
  }

  // Add statistics for numeric columns
  if (!injectAll && state.rows.length > 0) {
    lines.push("");
    lines.push("**Column Statistics**:");

    for (let colIdx = 0; colIdx < state.columns.length; colIdx++) {
      const col = state.columns[colIdx];
      const values = state.rows
        .map((row) => row[colIdx])
        .filter((cell) => cell.type === "Number")
        .map((cell) => (cell as { type: "Number"; value: number }).value);

      if (values.length > 0) {
        const sum = values.reduce((a, b) => a + b, 0);
        const avg = sum / values.length;
        const min = Math.min(...values);
        const max = Math.max(...values);
        lines.push(`- ${col.header || col.name}: sum=${sum.toLocaleString()}, avg=${avg.toFixed(2)}, min=${min}, max=${max}`);
      }
    }
  }

  lines.push("");
  lines.push("**Available Excel Tools** (use file_path: `" + state.filePath + "`):");
  lines.push("- `excel_filter`: Filter rows by column conditions");
  lines.push("- `excel_search`: Search for values across columns");
  lines.push("- `excel_aggregate`: Compute statistics (sum, avg, min, max, count)");
  lines.push("");

  return lines.join("\n");
}

/**
 * Check if Excel data is available for context injection
 */
function hasExcelContext(): boolean {
  const state = excelState();
  return !!(state.filePath && state.rows.length > 0);
}

// ==================== Event Handlers ====================

function handleFileChange(event: FileChangeEvent): void {
  const state = excelState();
  if (event.path === state.filePath) {
    setExcelState((prev) => ({
      ...prev,
      hasFileChanged: true,
    }));
  }
}

// ==================== Store Export ====================

export function useDataPanels() {
  // Set up event listeners
  let unlistenFileChange: (() => void) | null = null;
  let unlistenEditsApplied: (() => void) | null = null;

  // Initialize listeners
  const setupListeners = async () => {
    unlistenFileChange = await listenToFileChange(handleFileChange);
    unlistenEditsApplied = await listenToEditsApplied((event) => {
      console.log("Edits applied:", event);
    });
  };

  setupListeners();

  // Cleanup
  onCleanup(() => {
    unlistenFileChange?.();
    unlistenEditsApplied?.();
  });

  return {
    // State
    showDataPanels,
    setShowDataPanels,
    activeProvider,
    setActiveProvider,
    excelState,

    // Excel Actions
    openExcelFile,
    selectSheet,
    loadMoreRows,
    refreshData,
    validateData,
    addPendingEdit,
    removePendingEdit,
    clearPendingEdits,
    applyEdits,
    toggleWatch,
    setSchema,

    // Persistence
    loadPanelState,
    savePanelState,

    // Helpers
    getCellDisplayValue,

    // LLM Context
    generateExcelContext,
    hasExcelContext,
  };
}

// Export for use outside of SolidJS components
export {
  showDataPanels,
  setShowDataPanels,
  activeProvider,
  setActiveProvider,
  excelState,
  openExcelFile,
  selectSheet,
  loadMoreRows,
  refreshData,
  validateData,
  addPendingEdit,
  removePendingEdit,
  clearPendingEdits,
  applyEdits,
  toggleWatch,
  setSchema,
  loadPanelState,
  savePanelState,
  generateExcelContext,
  hasExcelContext,
};
