import { Component, Show } from "solid-js";
import type { FileSelectorProps } from "../types";

export const FileSelector: Component<FileSelectorProps> = (props) => {
  const fileName = () => {
    if (!props.filePath) return null;
    return props.filePath.split("/").pop() || props.filePath;
  };

  const formatSyncTime = () => {
    if (!props.lastSyncTime) return null;
    const diff = Date.now() - props.lastSyncTime;
    const minutes = Math.floor(diff / 60000);
    if (minutes < 1) return "just now";
    if (minutes === 1) return "1m ago";
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours === 1) return "1h ago";
    return `${hours}h ago`;
  };

  return (
    <div class="file-selector">
      <div class="file-info">
        <button
          class="file-button"
          onClick={() => props.onOpenFile()}
          title={props.filePath || "Open file"}
        >
          <span class="file-icon">{"\u{1F4C4}"}</span>
          <span class="file-name">{fileName() || "No file selected"}</span>
          <span class="dropdown-icon">{"\u{25BC}"}</span>
        </button>
        <Show when={props.recentFiles.length > 0}>
          <div class="recent-dropdown">
            {props.recentFiles.map((file) => (
              <button
                class="dropdown-item"
                onClick={() => props.onOpenFile(file)}
              >
                {file.split("/").pop()}
              </button>
            ))}
          </div>
        </Show>
      </div>
      <div class="sync-info">
        <Show when={props.hasFileChanged}>
          <span class="file-changed-indicator" title="File changed externally">
            {"\u{1F534}"}
          </span>
        </Show>
        <Show when={props.lastSyncTime}>
          <span class="sync-time" title="Last synced">
            {formatSyncTime()}
          </span>
        </Show>
        <button
          class="refresh-icon-btn"
          onClick={props.onRefresh}
          title="Refresh data"
        >
          {"\u{1F504}"}
        </button>
      </div>
    </div>
  );
};

// CSS is included inline for simplicity
const styles = `
.file-selector {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem;
  border-bottom: 1px solid var(--border);
  background: var(--card);
}

.file-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
  position: relative;
}

.file-button {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.625rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
  color: var(--foreground);
  cursor: pointer;
  max-width: 200px;
  transition: all var(--transition-fast);
}

.file-button:hover {
  border-color: var(--primary);
}

.file-icon {
  font-size: 0.875rem;
  flex-shrink: 0;
}

.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dropdown-icon {
  font-size: 0.5rem;
  opacity: 0.6;
  flex-shrink: 0;
}

.recent-dropdown {
  display: none;
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 0.25rem;
  min-width: 180px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow-md);
  z-index: 10;
}

.file-info:hover .recent-dropdown {
  display: block;
}

.dropdown-item {
  display: block;
  width: 100%;
  padding: 0.5rem 0.75rem;
  font-size: 0.8125rem;
  text-align: left;
  color: var(--foreground);
  background: transparent;
  border: none;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dropdown-item:hover {
  background: var(--accent);
}

.sync-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.file-changed-indicator {
  font-size: 0.5rem;
  animation: pulse 1s infinite;
}

.sync-time {
  font-size: 0.6875rem;
  color: var(--muted-foreground);
}

.refresh-icon-btn {
  padding: 0.25rem;
  background: transparent;
  border: none;
  font-size: 0.875rem;
  cursor: pointer;
  opacity: 0.6;
  transition: all var(--transition-fast);
}

.refresh-icon-btn:hover {
  opacity: 1;
}
`;

// Inject styles
if (typeof document !== "undefined") {
  const styleEl = document.createElement("style");
  styleEl.textContent = styles;
  document.head.appendChild(styleEl);
}

export default FileSelector;
