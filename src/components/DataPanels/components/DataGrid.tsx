import { Component, For, Show, createSignal, createEffect } from "solid-js";
import { getCellDisplayValue, columnIndexToLetter } from "../../../lib/excel-api";
import type { DataGridProps } from "../types";
import { ValidationBadge } from "./ValidationBadge";

export const DataGrid: Component<DataGridProps> = (props) => {
  let gridRef: HTMLDivElement | undefined;
  const [editingCell, setEditingCell] = createSignal<{ row: number; col: number } | null>(null);
  const [editValue, setEditValue] = createSignal("");

  // Check if a cell has an error
  const getCellError = (row: number, col: number) => {
    return props.validationErrors.find(
      (e) => e.row === row && e.col === col
    );
  };

  // Check if a cell has a pending edit
  const getCellEdit = (row: number, col: number) => {
    return props.pendingEdits.get(`${row},${col}`);
  };

  // Start editing a cell
  const startEditing = (row: number, col: number, currentValue: string) => {
    setEditingCell({ row, col });
    setEditValue(currentValue);
  };

  // Finish editing
  const finishEditing = () => {
    const cell = editingCell();
    if (cell) {
      props.onCellEdit(cell.row, cell.col, editValue());
      setEditingCell(null);
    }
  };

  // Cancel editing
  const cancelEditing = () => {
    setEditingCell(null);
    setEditValue("");
  };

  // Handle keyboard in edit mode
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      finishEditing();
    } else if (e.key === "Escape") {
      cancelEditing();
    }
  };

  // Handle scroll for infinite loading
  const handleScroll = (e: Event) => {
    const target = e.target as HTMLDivElement;
    const scrolledToBottom =
      target.scrollHeight - target.scrollTop <= target.clientHeight + 100;

    if (scrolledToBottom && props.hasMore && !props.isLoading) {
      props.onLoadMore();
    }
  };

  return (
    <div class="data-grid-container" ref={gridRef} onScroll={handleScroll}>
      <Show when={props.columns.length > 0} fallback={
        <div class="grid-empty">
          <p>No data to display</p>
        </div>
      }>
        <table class="data-grid">
          <thead>
            <tr>
              <th class="row-number-header">#</th>
              <For each={props.columns}>
                {(col) => (
                  <th class="column-header" title={col.header || col.name}>
                    <span class="col-letter">{col.name}</span>
                    <Show when={col.header}>
                      <span class="col-header">{col.header}</span>
                    </Show>
                  </th>
                )}
              </For>
            </tr>
          </thead>
          <tbody>
            <For each={props.rows}>
              {(row, rowIdx) => {
                const actualRow = props.offset + rowIdx();
                return (
                  <tr>
                    <td class="row-number">{actualRow + 1}</td>
                    <For each={row}>
                      {(cell, colIdx) => {
                        const error = () => getCellError(actualRow, colIdx());
                        const edit = () => getCellEdit(actualRow, colIdx());
                        const isEditing = () => {
                          const e = editingCell();
                          return e && e.row === actualRow && e.col === colIdx();
                        };
                        const displayValue = () => {
                          const e = edit();
                          if (e) return getCellDisplayValue(e.value);
                          return getCellDisplayValue(cell);
                        };

                        return (
                          <td
                            class={`data-cell ${error() ? "has-error" : ""} ${edit() ? "has-edit" : ""}`}
                            onDblClick={() => startEditing(actualRow, colIdx(), displayValue())}
                          >
                            <Show
                              when={isEditing()}
                              fallback={
                                <>
                                  <span class="cell-value">{displayValue()}</span>
                                  <Show when={error()}>
                                    <ValidationBadge
                                      type="error"
                                      message={error()!.message}
                                    />
                                  </Show>
                                  <Show when={edit() && !error()}>
                                    <span class="edit-indicator" title="Pending edit">
                                      *
                                    </span>
                                  </Show>
                                </>
                              }
                            >
                              <input
                                type="text"
                                class="cell-input"
                                value={editValue()}
                                onInput={(e) => setEditValue(e.currentTarget.value)}
                                onKeyDown={handleKeyDown}
                                onBlur={finishEditing}
                                autofocus
                              />
                            </Show>
                          </td>
                        );
                      }}
                    </For>
                  </tr>
                );
              }}
            </For>
          </tbody>
        </table>

        <Show when={props.isLoading}>
          <div class="loading-indicator">
            <span class="loading-spinner"></span>
            Loading...
          </div>
        </Show>

        <Show when={props.hasMore && !props.isLoading}>
          <div class="load-more">
            <button onClick={props.onLoadMore}>
              Load more rows ({props.totalRows - props.offset - props.rows.length} remaining)
            </button>
          </div>
        </Show>
      </Show>
    </div>
  );
};

// CSS is included inline
const styles = `
.data-grid-container {
  flex: 1;
  overflow: auto;
  background: var(--background);
}

.grid-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--muted-foreground);
  font-size: 0.875rem;
}

.data-grid {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.75rem;
  font-family: 'SF Mono', 'Monaco', monospace;
}

.data-grid thead {
  position: sticky;
  top: 0;
  z-index: 2;
}

.data-grid th {
  background: var(--card);
  border: 1px solid var(--border);
  padding: 0.375rem 0.5rem;
  text-align: left;
  font-weight: 600;
  white-space: nowrap;
}

.row-number-header {
  width: 40px;
  text-align: center;
  color: var(--muted-foreground);
}

.column-header {
  min-width: 80px;
  max-width: 200px;
}

.col-letter {
  display: block;
  font-size: 0.625rem;
  color: var(--muted-foreground);
  text-transform: uppercase;
}

.col-header {
  display: block;
  font-size: 0.75rem;
  color: var(--foreground);
  overflow: hidden;
  text-overflow: ellipsis;
}

.data-grid td {
  border: 1px solid var(--border);
  padding: 0.25rem 0.5rem;
  max-width: 200px;
  position: relative;
}

.row-number {
  text-align: center;
  color: var(--muted-foreground);
  background: var(--card);
  font-size: 0.625rem;
}

.data-cell {
  position: relative;
  cursor: pointer;
  transition: background var(--transition-fast);
}

.data-cell:hover {
  background: var(--accent);
}

.data-cell.has-error {
  background: rgba(220, 53, 69, 0.05);
}

.data-cell.has-edit {
  background: rgba(59, 130, 246, 0.08);
}

.cell-value {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cell-input {
  width: 100%;
  padding: 0;
  margin: -0.25rem -0.5rem;
  border: 2px solid var(--primary);
  border-radius: 0;
  font-size: inherit;
  font-family: inherit;
  background: var(--card);
}

.cell-input:focus {
  outline: none;
}

.edit-indicator {
  position: absolute;
  top: 0;
  right: 2px;
  color: #3b82f6;
  font-weight: bold;
  font-size: 0.625rem;
}

.loading-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 1rem;
  color: var(--muted-foreground);
  font-size: 0.8125rem;
}

.loading-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--border);
  border-top-color: var(--primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.load-more {
  padding: 0.75rem;
  text-align: center;
}

.load-more button {
  padding: 0.5rem 1rem;
  font-size: 0.75rem;
  color: var(--primary);
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.load-more button:hover {
  background: var(--card);
  border-color: var(--primary);
}
`;

// Inject styles
if (typeof document !== "undefined") {
  const styleEl = document.createElement("style");
  styleEl.textContent = styles;
  document.head.appendChild(styleEl);
}

export default DataGrid;
