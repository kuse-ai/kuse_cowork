import { Component, Show } from "solid-js";
import {
  useDataPanels,
  activeProvider,
  setActiveProvider,
} from "../../stores/dataPanels";
import { ExcelPanel } from "./ExcelPanel";
import "./DataPanelsDock.css";

export const DataPanelsDock: Component = () => {
  const providers = [
    { id: "excel" as const, label: "Excel", icon: "table" },
    // Future providers:
    // { id: "sheets" as const, label: "Sheets", icon: "cloud" },
    // { id: "csv" as const, label: "CSV", icon: "file-text" },
  ];

  return (
    <div class="data-panels-dock">
      <div class="dock-header">
        <h2 class="dock-title">Data Panels</h2>
        <div class="provider-tabs">
          {providers.map((provider) => (
            <button
              class={`provider-tab ${activeProvider() === provider.id ? "active" : ""}`}
              onClick={() => setActiveProvider(provider.id)}
            >
              <span class="tab-icon">{getProviderIcon(provider.id)}</span>
              <span class="tab-label">{provider.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div class="dock-content">
        <Show when={activeProvider() === "excel"}>
          <ExcelPanel />
        </Show>
        <Show when={activeProvider() === "sheets"}>
          <div class="coming-soon">
            <span class="coming-soon-icon">sheets</span>
            <p>Google Sheets integration coming soon</p>
          </div>
        </Show>
        <Show when={activeProvider() === "csv"}>
          <div class="coming-soon">
            <span class="coming-soon-icon">csv</span>
            <p>CSV support coming soon</p>
          </div>
        </Show>
      </div>
    </div>
  );
};

function getProviderIcon(provider: string): string {
  switch (provider) {
    case "excel":
      return "\u{1F4CA}"; // bar chart emoji
    case "sheets":
      return "\u{2601}"; // cloud emoji
    case "csv":
      return "\u{1F4C4}"; // page facing up emoji
    default:
      return "\u{1F4C1}"; // folder emoji
  }
}

export default DataPanelsDock;
