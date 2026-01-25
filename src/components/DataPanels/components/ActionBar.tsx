import { Component, Show } from "solid-js";
import type { ActionBarProps } from "../types";

export const ActionBar: Component<ActionBarProps> = (props) => {
  return (
    <div class="action-bar">
      <div class="action-row">
        <button
          class="action-btn secondary"
          onClick={props.onRefresh}
          disabled={props.isLoading}
          title="Refresh data"
        >
          <span class="btn-icon">{"\u{1F504}"}</span>
          Refresh
        </button>

        <button
          class="action-btn secondary"
          onClick={props.onValidate}
          disabled={props.isLoading || !props.hasSchema}
          title={props.hasSchema ? "Validate data" : "No schema defined"}
        >
          <span class="btn-icon">{"\u{2713}"}</span>
          Validate
        </button>

        <button
          class={`action-btn primary ${props.pendingEditCount > 0 ? "has-edits" : ""}`}
          onClick={props.onApply}
          disabled={!props.canApply}
          title={props.pendingEditCount > 0 ? `Apply ${props.pendingEditCount} edits` : "No pending edits"}
        >
          <span class="btn-icon">{"\u{2714}"}</span>
          Apply
          <Show when={props.pendingEditCount > 0}>
            <span class="edit-badge">{props.pendingEditCount}</span>
          </Show>
        </button>
      </div>

      <div class="action-row">
        <button
          class="action-btn ghost"
          onClick={props.onDownload}
          title="Download file"
        >
          <span class="btn-icon">{"\u{2B07}"}</span>
          Download
        </button>

        <button
          class="action-btn ghost"
          onClick={props.onOpenExternal}
          title="Open in Excel"
        >
          <span class="btn-icon">{"\u{1F4BB}"}</span>
          Open in Excel
        </button>

        <button
          class={`action-btn ghost toggle ${props.watchEnabled ? "active" : ""}`}
          onClick={props.onToggleWatch}
          title={props.watchEnabled ? "Stop watching for changes" : "Watch for external changes"}
        >
          <span class="btn-icon">{props.watchEnabled ? "\u{1F7E2}" : "\u{26AA}"}</span>
          Watch
        </button>
      </div>
    </div>
  );
};

// CSS
const styles = `
.action-bar {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem;
  border-top: 1px solid var(--border);
  background: var(--card);
}

.action-row {
  display: flex;
  gap: 0.5rem;
}

.action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.375rem;
  flex: 1;
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 500;
  border-radius: var(--radius);
  cursor: pointer;
  transition: all var(--transition-fast);
  position: relative;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn .btn-icon {
  font-size: 0.75rem;
}

.action-btn.primary {
  background: var(--primary);
  color: var(--primary-foreground);
  border: none;
}

.action-btn.primary:hover:not(:disabled) {
  opacity: 0.9;
}

.action-btn.primary.has-edits {
  animation: pulse-border 2s infinite;
}

@keyframes pulse-border {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(106, 64, 64, 0.3);
  }
  50% {
    box-shadow: 0 0 0 3px rgba(106, 64, 64, 0.1);
  }
}

.action-btn.secondary {
  background: var(--card);
  color: var(--foreground);
  border: 1px solid var(--border);
}

.action-btn.secondary:hover:not(:disabled) {
  background: var(--accent);
  border-color: var(--primary);
}

.action-btn.ghost {
  background: transparent;
  color: var(--muted-foreground);
  border: 1px solid transparent;
}

.action-btn.ghost:hover:not(:disabled) {
  color: var(--foreground);
  background: var(--accent);
}

.action-btn.toggle.active {
  color: #228b22;
}

.edit-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  font-size: 0.625rem;
  font-weight: 600;
  line-height: 16px;
  text-align: center;
  color: white;
  background: #3b82f6;
  border-radius: 8px;
}
`;

// Inject styles
if (typeof document !== "undefined") {
  const styleEl = document.createElement("style");
  styleEl.textContent = styles;
  document.head.appendChild(styleEl);
}

export default ActionBar;
