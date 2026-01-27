import { Component, For, Show, createEffect } from "solid-js";
import { useTraces, useSuggestions, Trace, Suggestion } from "../stores/traces";
import SuggestionCard from "./SuggestionCard";
import "./TracePanel.css";

interface TracePanelProps {
  docId: string | null;
  onClose?: () => void;
}

const TracePanel: Component<TracePanelProps> = (props) => {
  const {
    traces,
    traceSettings,
    isLoadingTraces,
    loadTraces,
    clearTraces,
    updateSettings,
    loadMoreTraces,
  } = useTraces();

  const {
    suggestions,
    isGeneratingSuggestions,
    isApplyingSuggestion,
    loadSuggestions,
    generateSuggestions,
    applySuggestion,
    rejectSuggestion,
    dismissSuggestion,
  } = useSuggestions();

  // Load traces when docId changes
  createEffect(() => {
    const docId = props.docId;
    if (docId) {
      loadTraces(docId, 100);
      loadSuggestions(docId, "pending");
    }
  });

  const getTraceIcon = (eventType: string): string => {
    switch (eventType) {
      case "edit":
        return "pencil";
      case "browse":
        return "globe";
      case "search":
        return "search";
      case "tool_start":
        return "play";
      case "tool_end":
        return "check";
      case "approval":
        return "thumbs-up";
      default:
        return "circle";
    }
  };

  const getTraceLabel = (trace: Trace): string => {
    switch (trace.event_type) {
      case "edit": {
        const delta = trace.delta;
        const deltaStr = delta !== null ? (delta > 0 ? `+${delta}` : `${delta}`) : "";
        const section = trace.section_path || "document";
        return `Edited ${section} ${deltaStr ? `(${deltaStr} chars)` : ""}`;
      }
      case "browse": {
        const url = trace.payload?.url as string;
        if (url) {
          try {
            const parsed = new URL(url);
            return `Visited ${parsed.hostname}${parsed.pathname.slice(0, 30)}`;
          } catch {
            return `Visited ${url.slice(0, 40)}`;
          }
        }
        return "Browsed";
      }
      case "search": {
        const query = trace.payload?.query as string;
        return query ? `Searched: "${query.slice(0, 30)}"` : "Searched";
      }
      case "tool_start": {
        const tool = trace.payload?.tool as string;
        return tool ? `Started: ${tool}` : "Started tool";
      }
      case "tool_end": {
        const tool = trace.payload?.tool as string;
        const success = trace.payload?.success as boolean;
        return tool
          ? `${success ? "Completed" : "Failed"}: ${tool}`
          : "Completed tool";
      }
      case "approval": {
        const action = trace.payload?.action as string;
        return action || "Action approved";
      }
      default:
        return trace.event_type;
    }
  };

  const formatTime = (timestamp: number): string => {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);

    if (minutes < 1) return "just now";
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return date.toLocaleDateString();
  };

  const handleToggleTracing = () => {
    updateSettings({ tracing_enabled: !traceSettings().tracing_enabled });
  };

  const handleClear = () => {
    const docId = props.docId;
    if (docId) {
      clearTraces(docId);
    }
  };

  const pendingSuggestions = () => suggestions().filter((s) => s.status === "pending");

  return (
    <div class="trace-panel">
      <div class="trace-panel-header">
        <h3>Activity Trace</h3>
        <div class="trace-header-actions">
          <button
            class={`trace-toggle-btn ${traceSettings().tracing_enabled ? "active" : ""}`}
            onClick={handleToggleTracing}
            title={traceSettings().tracing_enabled ? "Disable tracing" : "Enable tracing"}
          >
            {traceSettings().tracing_enabled ? "ON" : "OFF"}
          </button>
          <button
            class="trace-clear-btn"
            onClick={handleClear}
            title="Clear all traces"
          >
            Clear
          </button>
          {props.onClose && (
            <button class="trace-close-btn" onClick={props.onClose}>
              x
            </button>
          )}
        </div>
      </div>

      {/* Suggestions Section */}
      <div class="suggestions-section">
        <div class="suggestions-header">
          <span>AI Suggestions</span>
          <button
            class="generate-suggestions-btn"
            onClick={() => props.docId && generateSuggestions(props.docId)}
            disabled={!props.docId || isGeneratingSuggestions()}
          >
            {isGeneratingSuggestions() ? "..." : "Generate"}
          </button>
        </div>
        <Show
          when={pendingSuggestions().length > 0}
          fallback={
            <div class="suggestions-empty">
              <p>No suggestions yet</p>
              <p class="hint">Click "Generate" to get AI-powered suggestions</p>
            </div>
          }
        >
          <For each={pendingSuggestions()}>
            {(suggestion) => (
              <SuggestionCard
                suggestion={suggestion}
                onApprove={() => applySuggestion(suggestion.id)}
                onReject={() => rejectSuggestion(suggestion.id)}
                onDismiss={() => dismissSuggestion(suggestion.id)}
                isApplying={isApplyingSuggestion()}
              />
            )}
          </For>
        </Show>
      </div>

      {/* Traces Section */}
      <div class="traces-section">
        <Show
          when={!isLoadingTraces()}
          fallback={
            <div class="traces-loading">Loading traces...</div>
          }
        >
          <Show
            when={traces().length > 0}
            fallback={
              <div class="traces-empty">
                <p>No activity recorded yet</p>
                <p class="hint">
                  {traceSettings().tracing_enabled
                    ? "Your actions will appear here"
                    : "Enable tracing to record activity"}
                </p>
              </div>
            }
          >
            <div class="traces-list">
              <For each={traces()}>
                {(trace) => (
                  <TraceChip trace={trace} getIcon={getTraceIcon} getLabel={getTraceLabel} formatTime={formatTime} />
                )}
              </For>
            </div>
            <Show when={traces().length >= 50}>
              <button class="load-more-btn" onClick={loadMoreTraces}>
                Load more
              </button>
            </Show>
          </Show>
        </Show>
      </div>
    </div>
  );
};

interface TraceChipProps {
  trace: Trace;
  getIcon: (eventType: string) => string;
  getLabel: (trace: Trace) => string;
  formatTime: (timestamp: number) => string;
}

const TraceChip: Component<TraceChipProps> = (props) => {
  return (
    <div class={`trace-chip ${props.trace.event_type}`}>
      <span class="trace-chip-icon">{getIconEmoji(props.getIcon(props.trace.event_type))}</span>
      <div class="trace-chip-content">
        <span class="trace-chip-label">{props.getLabel(props.trace)}</span>
        <span class="trace-chip-time">{props.formatTime(props.trace.created_at)}</span>
      </div>
    </div>
  );
};

const getIconEmoji = (icon: string): string => {
  switch (icon) {
    case "pencil":
      return "P";
    case "globe":
      return "G";
    case "search":
      return "S";
    case "play":
      return ">";
    case "check":
      return "V";
    case "thumbs-up":
      return "+";
    default:
      return "o";
  }
};

export default TracePanel;
