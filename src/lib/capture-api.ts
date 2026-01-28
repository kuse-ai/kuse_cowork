import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri-api";

// ==================== Capture Types ====================

export type SourceType = "webpage" | "clipboard" | "ai_exchange" | "search" | "document";
export type ContributionType = "direct_copy" | "referenced" | "inspired" | "ai_assisted";

export interface ClipboardCapture {
  id: string;
  content_hash: string;
  content_preview: string;
  source_url: string | null;
  source_title: string | null;
  captured_at: number;
}

export interface BrowseCapture {
  id: string;
  url: string;
  page_title: string | null;
  entered_at: number;
  left_at: number | null;
  scroll_depth_percent: number | null;
}

export interface SearchCapture {
  id: string;
  query: string;
  search_engine: string;
  result_clicked: string | null;
  timestamp: number;
}

export interface AIExchangeCapture {
  id: string;
  question_hash: string;
  question_preview: string;
  answer_hash: string;
  answer_preview: string;
  model: string;
  context_doc_id: string | null;
  timestamp: number;
}

export interface DocEditCapture {
  id: string;
  doc_id: string;
  doc_title: string;
  edit_preview: string;
  char_delta: number;
  started_at: number;
  ended_at: number;
}

export interface ActiveSourceEntry {
  source_type: SourceType;
  source_id: string;
  title: string | null;
  activated_at: number;
  relevance: number;
}

export interface SourceLink {
  id: string;
  doc_id: string;
  section_path: string | null;
  content_hash: string;
  content_preview: string | null;
  created_at: number;
  confidence_score: number;
}

export interface LinkedSource {
  id: string;
  link_id: string;
  source_type: SourceType;
  source_id: string;
  contribution_type: ContributionType;
  timestamp: number;
}

export interface SourceLinkWithSources {
  link: SourceLink;
  sources: LinkedSource[];
}

export interface CaptureConfig {
  clipboard_enabled: boolean;
  browse_enabled: boolean;
  search_enabled: boolean;
  ai_exchange_enabled: boolean;
  source_linking_enabled: boolean;
  flush_interval_secs: number;
  clipboard_poll_ms: number;
}

export interface BatchInsertResult {
  clipboard_inserted: number;
  browse_inserted: number;
  search_inserted: number;
  ai_exchange_inserted: number;
  doc_edit_inserted: number;
}

// ==================== Search Engine Detection ====================

const SEARCH_ENGINES: Record<string, { name: string; queryParam: string }> = {
  "google.com": { name: "Google", queryParam: "q" },
  "bing.com": { name: "Bing", queryParam: "q" },
  "duckduckgo.com": { name: "DuckDuckGo", queryParam: "q" },
  "yahoo.com": { name: "Yahoo", queryParam: "p" },
  "baidu.com": { name: "Baidu", queryParam: "wd" },
  "yandex.com": { name: "Yandex", queryParam: "text" },
};

/**
 * Detect if URL is a search results page and extract query
 */
export function detectSearchQuery(url: string): { engine: string; query: string } | null {
  try {
    const parsed = new URL(url);
    const hostname = parsed.hostname.replace(/^www\./, "");

    for (const [domain, { name, queryParam }] of Object.entries(SEARCH_ENGINES)) {
      if (hostname.includes(domain)) {
        const query = parsed.searchParams.get(queryParam);
        if (query) {
          return { engine: name, query };
        }
      }
    }
  } catch {
    // Invalid URL
  }
  return null;
}

// ==================== Local Storage Fallback ====================

const BROWSE_STORAGE_KEY = "kuse-capture-browse";
const SEARCH_STORAGE_KEY = "kuse-capture-search";
const AI_STORAGE_KEY = "kuse-capture-ai";

function getLocalBrowse(): BrowseCapture[] {
  const stored = localStorage.getItem(BROWSE_STORAGE_KEY);
  return stored ? JSON.parse(stored) : [];
}

function saveLocalBrowse(captures: BrowseCapture[]) {
  localStorage.setItem(BROWSE_STORAGE_KEY, JSON.stringify(captures.slice(0, 100)));
}

function getLocalSearch(): SearchCapture[] {
  const stored = localStorage.getItem(SEARCH_STORAGE_KEY);
  return stored ? JSON.parse(stored) : [];
}

function saveLocalSearch(captures: SearchCapture[]) {
  localStorage.setItem(SEARCH_STORAGE_KEY, JSON.stringify(captures.slice(0, 50)));
}

function getLocalAI(): AIExchangeCapture[] {
  const stored = localStorage.getItem(AI_STORAGE_KEY);
  return stored ? JSON.parse(stored) : [];
}

function saveLocalAI(captures: AIExchangeCapture[]) {
  localStorage.setItem(AI_STORAGE_KEY, JSON.stringify(captures.slice(0, 20)));
}

// ==================== Page Context API ====================

/**
 * Report entering a page
 */
export async function reportPageContext(
  url: string,
  title: string | null,
  enteredAt: number = Date.now()
): Promise<string> {
  console.log("[capture-api] reportPageContext:", { url, title, enteredAt, isTauri: isTauri() });
  if (!isTauri()) {
    const id = crypto.randomUUID();
    const captures = getLocalBrowse();
    captures.unshift({
      id,
      url,
      page_title: title,
      entered_at: enteredAt,
      left_at: null,
      scroll_depth_percent: null,
    });
    saveLocalBrowse(captures);
    console.log("[capture-api] Stored locally (non-Tauri), id:", id);
    return id;
  }
  const result = await invoke<string>("report_page_context", { url, title, enteredAt });
  console.log("[capture-api] Tauri invoke returned:", result);
  return result;
}

/**
 * Update page context when leaving
 */
export async function updatePageContext(
  browseId: string,
  leftAt: number = Date.now(),
  scrollDepthPercent?: number
): Promise<boolean> {
  if (!isTauri()) {
    const captures = getLocalBrowse();
    const capture = captures.find((c) => c.id === browseId);
    if (capture) {
      capture.left_at = leftAt;
      capture.scroll_depth_percent = scrollDepthPercent ?? null;
      saveLocalBrowse(captures);
      return true;
    }
    return false;
  }
  return invoke<boolean>("update_page_context", { browseId, leftAt, scrollDepthPercent });
}

// ==================== Search Capture API ====================

/**
 * Capture a search query
 */
export async function captureSearch(
  query: string,
  searchEngine: string,
  timestamp: number = Date.now()
): Promise<string> {
  if (!isTauri()) {
    const id = crypto.randomUUID();
    const captures = getLocalSearch();
    captures.unshift({
      id,
      query,
      search_engine: searchEngine,
      result_clicked: null,
      timestamp,
    });
    saveLocalSearch(captures);
    return id;
  }
  return invoke<string>("capture_search", { query, searchEngine, timestamp });
}

/**
 * Update search with clicked result
 */
export async function updateSearchClick(searchId: string, clickedUrl: string): Promise<boolean> {
  if (!isTauri()) {
    const captures = getLocalSearch();
    const capture = captures.find((c) => c.id === searchId);
    if (capture) {
      capture.result_clicked = clickedUrl;
      saveLocalSearch(captures);
      return true;
    }
    return false;
  }
  return invoke<boolean>("update_search_click", { searchId, clickedUrl });
}

// ==================== AI Exchange Capture API ====================

/**
 * Capture an AI exchange (question + answer)
 */
export async function captureAIExchange(
  question: string,
  answer: string,
  model: string,
  contextDocId?: string
): Promise<string> {
  if (!isTauri()) {
    const id = crypto.randomUUID();
    const captures = getLocalAI();
    captures.unshift({
      id,
      question_hash: simpleHash(question),
      question_preview: question.slice(0, 200),
      answer_hash: simpleHash(answer),
      answer_preview: answer.slice(0, 300),
      model,
      context_doc_id: contextDocId ?? null,
      timestamp: Date.now(),
    });
    saveLocalAI(captures);
    return id;
  }
  return invoke<string>("capture_ai_exchange", { question, answer, model, contextDocId });
}

// Simple hash for browser fallback
function simpleHash(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash;
  }
  return Math.abs(hash).toString(16);
}

// ==================== Buffer Flush API ====================

/**
 * Flush the capture buffer to database
 */
export async function flushCaptureBuffer(): Promise<BatchInsertResult> {
  console.log("[capture-api] flushCaptureBuffer called, isTauri:", isTauri());
  if (!isTauri()) {
    return {
      clipboard_inserted: 0,
      browse_inserted: 0,
      search_inserted: 0,
      ai_exchange_inserted: 0,
    };
  }
  const result = await invoke<BatchInsertResult>("flush_capture_buffer");
  console.log("[capture-api] flush result:", result);
  return result;
}

// ==================== Source Linking API ====================

/**
 * Activate a source for tracking
 */
export async function activateSource(
  sourceType: SourceType,
  sourceId: string,
  title?: string
): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("activate_source", { sourceType, sourceId, title });
}

/**
 * Deactivate a source
 */
export async function deactivateSource(sourceId: string): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("deactivate_source", { sourceId });
}

/**
 * Create a source link connecting document content to active sources
 */
export async function createSourceLink(
  docId: string,
  content: string,
  sectionPath?: string
): Promise<SourceLinkWithSources> {
  if (!isTauri()) {
    return {
      link: {
        id: crypto.randomUUID(),
        doc_id: docId,
        section_path: sectionPath ?? null,
        content_hash: simpleHash(content),
        content_preview: content.slice(0, 200),
        created_at: Date.now(),
        confidence_score: 0,
      },
      sources: [],
    };
  }
  return invoke<SourceLinkWithSources>("create_source_link", { docId, sectionPath, content });
}

/**
 * Get document provenance (source links for a document)
 */
export async function getDocumentProvenance(docId: string): Promise<SourceLinkWithSources[]> {
  if (!isTauri()) return [];
  return invoke<SourceLinkWithSources[]>("get_document_provenance", { docId });
}

// ==================== Config API ====================

/**
 * Get capture configuration
 */
export async function getCaptureConfig(): Promise<CaptureConfig> {
  if (!isTauri()) {
    return {
      clipboard_enabled: true,
      browse_enabled: true,
      search_enabled: true,
      ai_exchange_enabled: true,
      source_linking_enabled: true,
      flush_interval_secs: 30,
      clipboard_poll_ms: 500,
    };
  }
  return invoke<CaptureConfig>("get_capture_config");
}

/**
 * Update capture configuration
 */
export async function updateCaptureConfig(config: CaptureConfig): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("update_capture_config", { config });
}

// ==================== Document Edit Capture API ====================

/**
 * Capture a document edit session
 */
export async function captureDocEdit(
  docId: string,
  docTitle: string,
  editPreview: string,
  charDelta: number,
  startedAt: number,
  endedAt: number
): Promise<string> {
  console.log("[capture-api] captureDocEdit:", { docId, docTitle, charDelta });
  if (!isTauri()) {
    return crypto.randomUUID();
  }
  return invoke<string>("capture_doc_edit", {
    docId,
    docTitle,
    editPreview,
    charDelta,
    startedAt,
    endedAt,
  });
}

/**
 * Get recent document edit captures
 */
export async function getRecentDocEdit(limit: number = 50): Promise<DocEditCapture[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<DocEditCapture[]>("get_recent_doc_edit", { limit });
}

// ==================== Clipboard API ====================

/**
 * Start clipboard monitoring
 */
export async function startClipboardMonitor(): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("start_clipboard_monitor");
}

/**
 * Stop clipboard monitoring
 */
export async function stopClipboardMonitor(): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("stop_clipboard_monitor");
}

/**
 * Set the active clipboard source (call when user is viewing a page)
 */
export async function setClipboardSource(
  url: string | null,
  title: string | null
): Promise<void> {
  console.log("[capture-api] setClipboardSource:", { url, title });
  if (!isTauri()) return;
  return invoke<void>("set_clipboard_source", { url, title });
}

/**
 * Get recent clipboard captures
 */
export async function getRecentClipboard(limit: number = 50): Promise<ClipboardCapture[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<ClipboardCapture[]>("get_recent_clipboard", { limit });
}

// ==================== Export & Clear API ====================

/**
 * Export all captures to a file, then clear the database
 * Returns the path to the exported file
 */
export async function exportAndClearCaptures(): Promise<string | null> {
  if (!isTauri()) {
    // Clear local storage for non-Tauri
    localStorage.removeItem(BROWSE_STORAGE_KEY);
    localStorage.removeItem(SEARCH_STORAGE_KEY);
    localStorage.removeItem(AI_STORAGE_KEY);
    return null;
  }
  return invoke<string>("export_and_clear_captures");
}

// ==================== Flush Timer ====================

let flushInterval: ReturnType<typeof setInterval> | null = null;

/**
 * Start the automatic flush timer (30s default)
 */
export function startFlushTimer(intervalSecs: number = 30) {
  if (flushInterval) return;
  flushInterval = setInterval(() => {
    flushCaptureBuffer().catch(console.error);
  }, intervalSecs * 1000);
}

/**
 * Stop the automatic flush timer
 */
export function stopFlushTimer() {
  if (flushInterval) {
    clearInterval(flushInterval);
    flushInterval = null;
  }
}
