import { Component, For, Show, createSignal, createEffect, onMount } from "solid-js";
import { useCapture, SourceLinkWithSources } from "../stores/capture";
import {
  BrowseCapture,
  SearchCapture,
  AIExchangeCapture,
  ClipboardCapture,
  DocEditCapture,
  flushCaptureBuffer,
  getRecentClipboard,
  getRecentDocEdit,
  exportAndClearCaptures,
} from "../lib/capture-api";
import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "../lib/tauri-api";
import "./CapturePanel.css";

interface CapturePanelProps {
  docId: string | null;
  onClose?: () => void;
}

const CapturePanel: Component<CapturePanelProps> = (props) => {
  const { config, updateConfig, getProvenance, flush } = useCapture();

  const [browseCaptures, setBrowseCaptures] = createSignal<BrowseCapture[]>([]);
  const [searchCaptures, setSearchCaptures] = createSignal<SearchCapture[]>([]);
  const [aiCaptures, setAICaptures] = createSignal<AIExchangeCapture[]>([]);
  const [clipboardCaptures, setClipboardCaptures] = createSignal<ClipboardCapture[]>([]);
  const [docEditCaptures, setDocEditCaptures] = createSignal<DocEditCapture[]>([]);
  const [sourceLinks, setSourceLinks] = createSignal<SourceLinkWithSources[]>([]);
  const [isLoading, setIsLoading] = createSignal(false);
  const [activeTab, setActiveTab] = createSignal<"activity" | "sources">("activity");

  const [error, setError] = createSignal<string | null>(null);
  const [exportedPath, setExportedPath] = createSignal<string | null>(null);
  const [isExporting, setIsExporting] = createSignal(false);

  // Load captures
  const loadCaptures = async () => {
    if (!isTauri()) {
      console.log("[Capture] Not in Tauri environment");
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      console.log("[Capture] Flushing buffer...");
      // First flush any buffered data
      await flushCaptureBuffer();

      console.log("[Capture] Loading recent captures...");
      // Load recent captures from database
      const [browse, search, ai, clipboard, docEdit] = await Promise.all([
        invoke<BrowseCapture[]>("get_recent_browse", { limit: 50 }),
        invoke<SearchCapture[]>("get_recent_search", { limit: 30 }),
        invoke<AIExchangeCapture[]>("get_recent_ai_exchange", { limit: 20 }),
        getRecentClipboard(50),
        getRecentDocEdit(50),
      ]);

      console.log("[Capture] Loaded:", { browse: browse.length, search: search.length, ai: ai.length, clipboard: clipboard.length, docEdit: docEdit.length });

      setBrowseCaptures(browse || []);
      setSearchCaptures(search || []);
      setAICaptures(ai || []);
      setClipboardCaptures(clipboard || []);
      setDocEditCaptures(docEdit || []);

      // Load source links for current document
      if (props.docId) {
        const links = await getProvenance(props.docId);
        setSourceLinks(links || []);
      }
    } catch (e) {
      console.error("[Capture] Failed to load captures:", e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsLoading(false);
    }
  };

  // Load on mount and when docId changes
  onMount(loadCaptures);

  createEffect(() => {
    const docId = props.docId;
    if (docId) {
      loadCaptures();
    }
  });

  const handleToggleCapture = () => {
    const allEnabled =
      config().clipboard_enabled &&
      config().browse_enabled &&
      config().search_enabled &&
      config().ai_exchange_enabled;

    updateConfig({
      clipboard_enabled: !allEnabled,
      browse_enabled: !allEnabled,
      search_enabled: !allEnabled,
      ai_exchange_enabled: !allEnabled,
    });
  };

  const handleFlush = async () => {
    await flush();
    await loadCaptures();
  };

  const handleExportAndClear = async () => {
    if (isExporting()) return;
    setIsExporting(true);
    setError(null);
    setExportedPath(null);
    try {
      const path = await exportAndClearCaptures();
      if (path) {
        setExportedPath(path);
        // Reload to show empty state
        await loadCaptures();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsExporting(false);
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

  const formatDuration = (enteredAt: number, leftAt: number | null): string => {
    if (!leftAt) return "active";
    const secs = Math.floor((leftAt - enteredAt) / 1000);
    if (secs < 60) return `${secs}s`;
    const mins = Math.floor(secs / 60);
    return `${mins}m`;
  };

  const getHostname = (url: string): string => {
    try {
      return new URL(url).hostname.replace("www.", "");
    } catch {
      return url.slice(0, 30);
    }
  };

  const isEnabled = () =>
    config().clipboard_enabled ||
    config().browse_enabled ||
    config().search_enabled ||
    config().ai_exchange_enabled;

  // Combine all activities into a timeline
  const activityTimeline = () => {
    const items: Array<{
      type: "browse" | "search" | "ai" | "clipboard" | "edit";
      timestamp: number;
      data: BrowseCapture | SearchCapture | AIExchangeCapture | ClipboardCapture | DocEditCapture;
    }> = [];

    browseCaptures().forEach((b) =>
      items.push({ type: "browse", timestamp: b.entered_at, data: b })
    );
    searchCaptures().forEach((s) =>
      items.push({ type: "search", timestamp: s.timestamp, data: s })
    );
    aiCaptures().forEach((a) =>
      items.push({ type: "ai", timestamp: a.timestamp, data: a })
    );
    clipboardCaptures().forEach((c) =>
      items.push({ type: "clipboard", timestamp: c.captured_at, data: c })
    );
    docEditCaptures().forEach((d) =>
      items.push({ type: "edit", timestamp: d.ended_at, data: d })
    );

    return items.sort((a, b) => b.timestamp - a.timestamp).slice(0, 50);
  };

  return (
    <div class="capture-panel">
      <div class="capture-panel-header">
        <h3>Capture & Sources</h3>
        <div class="capture-header-actions">
          <button
            class={`capture-toggle-btn ${isEnabled() ? "active" : ""}`}
            onClick={handleToggleCapture}
            title={isEnabled() ? "Disable capture" : "Enable capture"}
          >
            {isEnabled() ? "ON" : "OFF"}
          </button>
          <button
            class="capture-flush-btn"
            onClick={handleFlush}
            title="Flush buffer to database"
          >
            Sync
          </button>
          <button
            class="capture-export-btn"
            onClick={handleExportAndClear}
            disabled={isExporting()}
            title="Export all captures to file and clear database"
          >
            {isExporting() ? "..." : "Export"}
          </button>
          {props.onClose && (
            <button class="capture-close-btn" onClick={props.onClose}>
              ×
            </button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div class="capture-tabs">
        <button
          class={`capture-tab ${activeTab() === "activity" ? "active" : ""}`}
          onClick={() => setActiveTab("activity")}
        >
          Activity
        </button>
        <button
          class={`capture-tab ${activeTab() === "sources" ? "active" : ""}`}
          onClick={() => setActiveTab("sources")}
        >
          Sources {sourceLinks().length > 0 && `(${sourceLinks().length})`}
        </button>
      </div>

      {/* Activity Tab */}
      <Show when={activeTab() === "activity"}>
        <div class="capture-section">
          <Show when={error()}>
            <div class="capture-error">Error: {error()}</div>
          </Show>
          <Show when={exportedPath()}>
            <div class="capture-success">
              Exported to: {exportedPath()!.split("/").pop()}
            </div>
          </Show>
          <Show
            when={!isLoading()}
            fallback={<div class="capture-loading">Loading...</div>}
          >
            <Show
              when={activityTimeline().length > 0}
              fallback={
                <div class="capture-empty">
                  <p>No activity captured yet</p>
                  <p class="hint">
                    {isEnabled()
                      ? "Open Browser panel and navigate, or start a new AI chat"
                      : "Enable capture to start tracking"}
                  </p>
                </div>
              }
            >
              <div class="capture-list">
                <For each={activityTimeline()}>
                  {(item) => (
                    <div class={`capture-item ${item.type}`}>
                      <span class="capture-icon">
                        {item.type === "browse" && "🌐"}
                        {item.type === "search" && "🔍"}
                        {item.type === "ai" && "🤖"}
                        {item.type === "clipboard" && "📋"}
                        {item.type === "edit" && "✏️"}
                      </span>
                      <div class="capture-content">
                        <Show when={item.type === "browse"}>
                          {(() => {
                            const b = item.data as BrowseCapture;
                            return (
                              <>
                                <span class="capture-title">
                                  {b.page_title || getHostname(b.url)}
                                </span>
                                <span class="capture-meta">
                                  {getHostname(b.url)} · {formatDuration(b.entered_at, b.left_at)}
                                </span>
                              </>
                            );
                          })()}
                        </Show>
                        <Show when={item.type === "search"}>
                          {(() => {
                            const s = item.data as SearchCapture;
                            return (
                              <>
                                <span class="capture-title">"{s.query}"</span>
                                <span class="capture-meta">
                                  {s.search_engine}
                                  {s.result_clicked && " → clicked result"}
                                </span>
                              </>
                            );
                          })()}
                        </Show>
                        <Show when={item.type === "ai"}>
                          {(() => {
                            const a = item.data as AIExchangeCapture;
                            return (
                              <>
                                <span class="capture-title">{a.question_preview}</span>
                                <span class="capture-meta">
                                  {a.model} · {a.answer_preview.slice(0, 50)}...
                                </span>
                              </>
                            );
                          })()}
                        </Show>
                        <Show when={item.type === "clipboard"}>
                          {(() => {
                            const c = item.data as ClipboardCapture;
                            return (
                              <>
                                <span class="capture-title">
                                  {c.content_preview.slice(0, 60)}
                                  {c.content_preview.length > 60 && "..."}
                                </span>
                                <span class="capture-meta">
                                  {c.source_url ? `from ${getHostname(c.source_url)}` : "copied"}
                                  {c.source_title && ` · ${c.source_title.slice(0, 30)}`}
                                </span>
                              </>
                            );
                          })()}
                        </Show>
                        <Show when={item.type === "edit"}>
                          {(() => {
                            const d = item.data as DocEditCapture;
                            const deltaStr = d.char_delta > 0 ? `+${d.char_delta}` : `${d.char_delta}`;
                            return (
                              <>
                                <span class="capture-title">{d.doc_title}</span>
                                <span class="capture-meta">
                                  {deltaStr} chars · {d.edit_preview.slice(0, 40)}
                                  {d.edit_preview.length > 40 && "..."}
                                </span>
                              </>
                            );
                          })()}
                        </Show>
                      </div>
                      <span class="capture-time">{formatTime(item.timestamp)}</span>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </Show>
        </div>
      </Show>

      {/* Sources Tab */}
      <Show when={activeTab() === "sources"}>
        <div class="capture-section">
          <Show
            when={props.docId}
            fallback={
              <div class="capture-empty">
                <p>Open a document to see its sources</p>
              </div>
            }
          >
            <Show
              when={sourceLinks().length > 0}
              fallback={
                <div class="capture-empty">
                  <p>No sources linked yet</p>
                  <p class="hint">Sources are linked when you save after researching</p>
                </div>
              }
            >
              <div class="source-links-list">
                <For each={sourceLinks()}>
                  {(link) => (
                    <div class="source-link-card">
                      <div class="source-link-header">
                        <span class="source-link-preview">
                          {link.link.content_preview?.slice(0, 100)}...
                        </span>
                        <span class="source-link-confidence">
                          {Math.round(link.link.confidence_score * 100)}% confidence
                        </span>
                      </div>
                      <div class="source-link-sources">
                        <For each={link.sources}>
                          {(source) => (
                            <span class={`source-tag ${source.source_type}`}>
                              {source.source_type === "webpage" && "🌐"}
                              {source.source_type === "ai_exchange" && "🤖"}
                              {source.source_type === "clipboard" && "📋"}
                              {source.source_type === "search" && "🔍"}
                              {source.contribution_type}
                            </span>
                          )}
                        </For>
                      </div>
                      <span class="source-link-time">{formatTime(link.link.created_at)}</span>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default CapturePanel;
