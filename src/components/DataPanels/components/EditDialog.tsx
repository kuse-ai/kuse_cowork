import { Component, Show, For } from "solid-js";
import { getCellDisplayValue, columnIndexToLetter } from "../../../lib/excel-api";
import type { EditDialogProps } from "../types";

export const EditDialog: Component<EditDialogProps> = (props) => {
  const edits = () => Array.from(props.pendingEdits.entries());

  return (
    <Show when={props.isOpen}>
      <div class="edit-dialog-overlay" onClick={props.onCancel}>
        <div class="edit-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="dialog-header">
            <h3>Review Changes</h3>
            <span class="edit-count">{props.pendingEdits.size} edits</span>
          </div>

          <div class="dialog-content">
            <div class="edits-list">
              <For each={edits()}>
                {([key, edit]) => (
                  <div class="edit-item">
                    <div class="edit-location">
                      {columnIndexToLetter(edit.col)}{edit.row + 1}
                    </div>
                    <div class="edit-values">
                      <span class="old-value">
                        {edit.original_value ? getCellDisplayValue(edit.original_value) : "(empty)"}
                      </span>
                      <span class="arrow">{"\u{2192}"}</span>
                      <span class="new-value">
                        {getCellDisplayValue(edit.value)}
                      </span>
                    </div>
                    <button
                      class="remove-edit-btn"
                      onClick={() => props.onRemoveEdit(edit.row, edit.col)}
                      title="Remove this edit"
                    >
                      {"\u{2715}"}
                    </button>
                  </div>
                )}
              </For>
            </div>
          </div>

          <div class="dialog-footer">
            <button class="dialog-btn secondary" onClick={props.onCancel}>
              Cancel
            </button>
            <button
              class="dialog-btn primary"
              onClick={props.onConfirm}
              disabled={props.pendingEdits.size === 0}
            >
              Apply Changes
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};

// CSS
const styles = `
.edit-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  backdrop-filter: blur(2px);
}

.edit-dialog {
  width: 90%;
  max-width: 480px;
  max-height: 80vh;
  background: var(--card);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid var(--border);
}

.dialog-header h3 {
  font-family: 'Instrument Serif', Georgia, serif;
  font-weight: 400;
  font-size: 1.125rem;
  color: var(--foreground);
  margin: 0;
}

.edit-count {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--primary);
  background: var(--accent);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-full);
}

.dialog-content {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem 1.25rem;
}

.edits-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.edit-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.625rem 0.75rem;
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}

.edit-location {
  font-family: 'SF Mono', 'Monaco', monospace;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--primary);
  min-width: 50px;
}

.edit-values {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8125rem;
  overflow: hidden;
}

.old-value {
  color: var(--muted-foreground);
  text-decoration: line-through;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.arrow {
  color: var(--muted-foreground);
  font-size: 0.75rem;
}

.new-value {
  color: var(--foreground);
  font-weight: 500;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remove-edit-btn {
  padding: 0.25rem;
  background: transparent;
  border: none;
  font-size: 0.75rem;
  color: var(--muted-foreground);
  cursor: pointer;
  transition: color var(--transition-fast);
}

.remove-edit-btn:hover {
  color: #dc3545;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
  border-top: 1px solid var(--border);
}

.dialog-btn {
  padding: 0.625rem 1.25rem;
  font-size: 0.875rem;
  font-weight: 500;
  border-radius: var(--radius);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.dialog-btn.primary {
  background: var(--primary);
  color: var(--primary-foreground);
  border: none;
}

.dialog-btn.primary:hover:not(:disabled) {
  opacity: 0.9;
}

.dialog-btn.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.dialog-btn.secondary {
  background: var(--card);
  color: var(--foreground);
  border: 1px solid var(--border);
}

.dialog-btn.secondary:hover {
  background: var(--accent);
}
`;

// Inject styles
if (typeof document !== "undefined") {
  const styleEl = document.createElement("style");
  styleEl.textContent = styles;
  document.head.appendChild(styleEl);
}

export default EditDialog;
