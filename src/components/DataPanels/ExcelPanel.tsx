import { Component, Show, createSignal, createMemo } from "solid-js";
import {
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
} from "../../stores/dataPanels";
import { FileSelector } from "./components/FileSelector";
import { SheetSelector } from "./components/SheetSelector";
import { DataGrid } from "./components/DataGrid";
import { ActionBar } from "./components/ActionBar";
import { EditDialog } from "./components/EditDialog";
import "./ExcelPanel.css";

export const ExcelPanel: Component = () => {
  const [showEditDialog, setShowEditDialog] = createSignal(false);

  const state = excelState;

  const pendingEditCount = createMemo(() => state().pendingEdits.size);
  const hasSchema = createMemo(() => state().schema !== null);
  const canApply = createMemo(() => pendingEditCount() > 0 && !state().isLoading);
  const hasMore = createMemo(() => {
    const s = state();
    return s.offset + s.rows.length < s.totalRows;
  });

  const handleRefresh = async () => {
    await refreshData();
  };

  const handleValidate = async () => {
    await validateData();
  };

  const handleApply = () => {
    if (pendingEditCount() > 0) {
      setShowEditDialog(true);
    }
  };

  const handleConfirmApply = async () => {
    await applyEdits();
    setShowEditDialog(false);
  };

  const handleDownload = () => {
    // Open the file in the default application
    const filePath = state().filePath;
    if (filePath) {
      // This would use shell.open in Tauri
      console.log("Download/Open:", filePath);
    }
  };

  const handleOpenExternal = async () => {
    const filePath = state().filePath;
    if (filePath) {
      try {
        const { open } = await import("@tauri-apps/plugin-shell");
        await open(filePath);
      } catch (error) {
        console.error("Failed to open file:", error);
      }
    }
  };

  return (
    <div class="excel-panel">
      <Show
        when={state().filePath}
        fallback={
          <div class="excel-panel-empty">
            <div class="empty-icon">{"\u{1F4CA}"}</div>
            <h3>No File Open</h3>
            <p>Open an Excel file to preview and edit data</p>
            <button class="open-file-btn" onClick={() => openExcelFile()}>
              Open Excel File
            </button>
            <Show when={state().recentFiles.length > 0}>
              <div class="recent-files">
                <span class="recent-label">Recent:</span>
                {state().recentFiles.slice(0, 3).map((file) => (
                  <button
                    class="recent-file-btn"
                    onClick={() => openExcelFile(file)}
                    title={file}
                  >
                    {file.split("/").pop()}
                  </button>
                ))}
              </div>
            </Show>
          </div>
        }
      >
        <div class="excel-panel-content">
          <FileSelector
            filePath={state().filePath}
            recentFiles={state().recentFiles}
            lastSyncTime={state().lastSyncTime}
            hasFileChanged={state().hasFileChanged}
            onOpenFile={openExcelFile}
            onRefresh={handleRefresh}
          />

          <SheetSelector
            sheets={state().sheets}
            activeSheet={state().activeSheet}
            activeRange={state().activeRange}
            onSheetSelect={selectSheet}
            onRangeChange={() => {}}
          />

          <Show when={state().error}>
            <div class="error-banner">
              <span class="error-icon">{"\u{26A0}"}</span>
              <span class="error-message">{state().error}</span>
            </div>
          </Show>

          <Show when={state().hasFileChanged}>
            <div class="file-changed-banner">
              <span class="changed-icon">{"\u{1F504}"}</span>
              <span>File has changed externally</span>
              <button class="refresh-btn" onClick={handleRefresh}>
                Refresh
              </button>
            </div>
          </Show>

          <DataGrid
            rows={state().rows}
            columns={state().columns}
            validationErrors={state().validationErrors}
            pendingEdits={state().pendingEdits}
            offset={state().offset}
            totalRows={state().totalRows}
            isLoading={state().isLoading}
            hasMore={hasMore()}
            onCellEdit={addPendingEdit}
            onLoadMore={loadMoreRows}
          />

          <ActionBar
            pendingEditCount={pendingEditCount()}
            isLoading={state().isLoading}
            watchEnabled={state().watchEnabled}
            hasSchema={hasSchema()}
            canApply={canApply()}
            onRefresh={handleRefresh}
            onValidate={handleValidate}
            onApply={handleApply}
            onDownload={handleDownload}
            onOpenExternal={handleOpenExternal}
            onToggleWatch={toggleWatch}
          />
        </div>
      </Show>

      <EditDialog
        isOpen={showEditDialog()}
        pendingEdits={state().pendingEdits}
        onConfirm={handleConfirmApply}
        onCancel={() => setShowEditDialog(false)}
        onRemoveEdit={removePendingEdit}
      />
    </div>
  );
};

export default ExcelPanel;
