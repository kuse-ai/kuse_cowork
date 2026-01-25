import { Component, For, Show } from "solid-js";
import type { SheetSelectorProps } from "../types";

export const SheetSelector: Component<SheetSelectorProps> = (props) => {
  return (
    <div class="sheet-selector">
      <div class="selector-group">
        <label class="selector-label">Sheet</label>
        <select
          class="sheet-select"
          value={props.activeSheet || ""}
          onChange={(e) => props.onSheetSelect(e.currentTarget.value)}
        >
          <For each={props.sheets}>
            {(sheet) => (
              <option value={sheet.name}>
                {sheet.name} ({sheet.row_count} rows)
              </option>
            )}
          </For>
        </select>
      </div>
      <div class="selector-group range-group">
        <label class="selector-label">Range</label>
        <input
          type="text"
          class="range-input"
          placeholder="e.g., A1:D100"
          value={props.activeRange || ""}
          onChange={(e) => {
            const value = e.currentTarget.value.trim();
            props.onRangeChange(value || null);
          }}
        />
      </div>
    </div>
  );
};

// CSS is included inline for simplicity
const styles = `
.sheet-selector {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.625rem 0.75rem;
  border-bottom: 1px solid var(--border);
  background: var(--card);
}

.selector-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.selector-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--muted-foreground);
  white-space: nowrap;
}

.sheet-select {
  padding: 0.375rem 0.625rem;
  font-size: 0.8125rem;
  color: var(--foreground);
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  min-width: 100px;
  max-width: 150px;
}

.sheet-select:focus {
  outline: none;
  border-color: var(--primary);
}

.range-group {
  flex: 1;
}

.range-input {
  flex: 1;
  padding: 0.375rem 0.625rem;
  font-size: 0.8125rem;
  font-family: 'SF Mono', 'Monaco', monospace;
  color: var(--foreground);
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  max-width: 100px;
}

.range-input:focus {
  outline: none;
  border-color: var(--primary);
}

.range-input::placeholder {
  color: var(--muted-foreground);
  opacity: 0.6;
}
`;

// Inject styles
if (typeof document !== "undefined") {
  const styleEl = document.createElement("style");
  styleEl.textContent = styles;
  document.head.appendChild(styleEl);
}

export default SheetSelector;
