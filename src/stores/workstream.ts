import { createSignal, onCleanup } from "solid-js";
import {
  WorkBlock,
  Timeline,
  CreateBlockInput,
  ManualBlockInput,
  UpdateBlockInput,
  WorkBlockQuery,
  createBlock,
  createManualBlock,
  updateBlock,
  deleteBlock,
  listBlocks,
  getTimeline,
  enhanceSummary,
  cleanup,
  getDisplaySummary,
  formatDuration,
  formatRelativeTime,
} from "../lib/workstream-api";

export type {
  WorkBlock,
  Timeline,
  CreateBlockInput,
  ManualBlockInput,
  UpdateBlockInput,
  WorkBlockQuery,
};

export { getDisplaySummary, formatDuration, formatRelativeTime };

// ==================== Content-Focused Event ====================

interface ContentEvent {
  id: string;
  timestamp: number;
  type: "write" | "research" | "tool" | "save";
  contextType: "document" | "browser" | "task" | "mixed";
  contextId?: string;
  contextTitle?: string;
  // The valuable part - actual content
  content: string;
  url?: string;
}

// ==================== State ====================

const [blocks, setBlocks] = createSignal<WorkBlock[]>([]);
const [isLoading, setIsLoading] = createSignal(false);
const [isEnhancing, setIsEnhancing] = createSignal(false);
const [contentBuffer, setContentBuffer] = createSignal<ContentEvent[]>([]);
const [currentContext, setCurrentContext] = createSignal<{
  type: string;
  id?: string;
  title?: string;
} | null>(null);

// Buffer settings
const INACTIVITY_THRESHOLD_MS = 3 * 60 * 1000; // 3 minutes - shorter for better granularity
const MIN_CONTENT_FOR_BLOCK = 1; // Even one meaningful content is worth saving
const MAX_BUFFER_SIZE = 20; // Smaller buffer, more frequent meaningful blocks

let inactivityTimer: number | null = null;

// ==================== Summary Generation ====================

function generateMeaningfulSummary(events: ContentEvent[]): string {
  if (events.length === 0) return "Brief activity";

  // Group by type
  const writes = events.filter((e) => e.type === "write");
  const research = events.filter((e) => e.type === "research");
  const tools = events.filter((e) => e.type === "tool");
  const saves = events.filter((e) => e.type === "save");

  const parts: string[] = [];

  // Summarize writing - use the most recent/longest content snippet
  if (writes.length > 0) {
    const bestWrite = writes.reduce((best, curr) =>
      curr.content.length > best.content.length ? curr : best
    );
    const preview = bestWrite.content.slice(0, 80);
    const title = bestWrite.contextTitle;
    if (saves.length > 0) {
      parts.push(`Wrote and saved: "${preview}${preview.length < bestWrite.content.length ? "..." : ""}"`);
    } else {
      parts.push(`Working on${title ? ` ${title}` : ""}: "${preview}${preview.length < bestWrite.content.length ? "..." : ""}"`);
    }
  }

  // Summarize research
  if (research.length > 0) {
    const topics = research
      .map((r) => r.contextTitle || r.content)
      .filter((t) => t.length > 0)
      .slice(0, 3);
    if (topics.length > 0) {
      if (parts.length > 0) {
        parts.push(`researched: ${topics.join(", ")}`);
      } else {
        parts.push(`Researched: ${topics.join(", ")}`);
      }
    }
  }

  // Summarize tool usage
  if (tools.length > 0 && parts.length === 0) {
    const toolNames = [...new Set(tools.map((t) => t.content))].slice(0, 3);
    parts.push(`Used tools: ${toolNames.join(", ")}`);
  }

  return parts.join("; ") || "Brief activity";
}

// ==================== Buffer Logic ====================

async function flushBuffer(): Promise<WorkBlock | null> {
  const buffer = contentBuffer();
  if (buffer.length < MIN_CONTENT_FOR_BLOCK) return null;

  const startedAt = buffer[0].timestamp;
  const endedAt = buffer[buffer.length - 1].timestamp;

  // Determine context
  const contextTypes = [...new Set(buffer.map((e) => e.contextType))];
  const contextType = contextTypes.length === 1 ? contextTypes[0] : "mixed";
  const contextId = buffer[0].contextId;
  const contextTitle = buffer.find((e) => e.contextTitle)?.contextTitle;

  // Count events
  const editCount = buffer.filter((e) => e.type === "write").length;
  const browseCount = buffer.filter((e) => e.type === "research").length;

  // Extract URLs
  const researchUrls = [
    ...new Set(buffer.filter((e) => e.url).map((e) => e.url!)),
  ].slice(0, 5);

  // Generate meaningful summary from actual content
  const autoSummary = generateMeaningfulSummary(buffer);

  const input: CreateBlockInput = {
    context_type: contextType,
    context_id: contextId,
    context_title: contextTitle,
    started_at: startedAt,
    ended_at: endedAt,
    auto_summary: autoSummary,
    edit_count: editCount,
    browse_count: browseCount,
    research_urls: researchUrls,
  };

  try {
    const block = await createBlock(input);
    setContentBuffer([]);
    setBlocks((prev) => [block, ...prev]);
    return block;
  } catch (e) {
    console.error("Failed to create work block:", e);
    return null;
  }
}

function startInactivityTimer() {
  if (inactivityTimer) {
    clearTimeout(inactivityTimer);
  }

  inactivityTimer = window.setTimeout(async () => {
    if (contentBuffer().length >= MIN_CONTENT_FOR_BLOCK) {
      await flushBuffer();
    }
  }, INACTIVITY_THRESHOLD_MS);
}

function stopInactivityTimer() {
  if (inactivityTimer) {
    clearTimeout(inactivityTimer);
    inactivityTimer = null;
  }
}

// ==================== Main Hook ====================

export function useWorkStream() {
  onCleanup(() => {
    stopInactivityTimer();
  });

  const loadTimeline = async (query?: WorkBlockQuery) => {
    setIsLoading(true);
    try {
      const timeline = await getTimeline(query?.limit || 50);
      setBlocks(timeline.blocks);
    } catch (e) {
      console.error("Failed to load timeline:", e);
    } finally {
      setIsLoading(false);
    }
  };

  // Track meaningful content - this is what gets called
  const trackContent = async (event: Omit<ContentEvent, "id" | "timestamp">) => {
    // Skip empty content
    if (!event.content || event.content.trim().length === 0) return;

    const contentEvent: ContentEvent = {
      ...event,
      id: crypto.randomUUID(),
      timestamp: Date.now(),
    };

    // Check for context switch
    const ctx = currentContext();
    const contextSwitched =
      ctx !== null &&
      (ctx.type !== event.contextType || ctx.id !== event.contextId);

    // Flush on context switch
    if (contextSwitched && contentBuffer().length >= MIN_CONTENT_FOR_BLOCK) {
      await flushBuffer();
    }

    // Update current context
    setCurrentContext({
      type: event.contextType,
      id: event.contextId,
      title: event.contextTitle,
    });

    // Add to buffer
    setContentBuffer((prev) => [...prev, contentEvent]);

    // Restart inactivity timer
    startInactivityTimer();

    // Force flush if buffer is full
    if (contentBuffer().length >= MAX_BUFFER_SIZE) {
      await flushBuffer();
    }
  };

  const createSnapshot = async (): Promise<WorkBlock | null> => {
    return flushBuffer();
  };

  const addManualEntry = async (input: ManualBlockInput): Promise<WorkBlock | null> => {
    try {
      const block = await createManualBlock(input);
      setBlocks((prev) => [block, ...prev]);
      return block;
    } catch (e) {
      console.error("Failed to create manual entry:", e);
      return null;
    }
  };

  const editBlock = async (id: string, input: UpdateBlockInput): Promise<WorkBlock | null> => {
    try {
      const updated = await updateBlock(id, input);
      if (updated) {
        setBlocks((prev) => prev.map((b) => (b.id === id ? updated : b)));
      }
      return updated;
    } catch (e) {
      console.error("Failed to update block:", e);
      return null;
    }
  };

  const removeBlock = async (id: string): Promise<boolean> => {
    try {
      const success = await deleteBlock(id);
      if (success) {
        setBlocks((prev) => prev.filter((b) => b.id !== id));
      }
      return success;
    } catch (e) {
      console.error("Failed to delete block:", e);
      return false;
    }
  };

  const pinBlock = async (id: string, pinned: boolean): Promise<void> => {
    await editBlock(id, { is_pinned: pinned });
  };

  const enhanceBlockSummary = async (id: string): Promise<string | null> => {
    setIsEnhancing(true);
    try {
      const summary = await enhanceSummary(id);
      const updated = await listBlocks({ limit: 50 });
      setBlocks(updated);
      return summary;
    } catch (e) {
      console.error("Failed to enhance summary:", e);
      return null;
    } finally {
      setIsEnhancing(false);
    }
  };

  const runCleanup = async () => {
    try {
      const result = await cleanup();
      if (result.blocks_deleted > 0) {
        await loadTimeline();
      }
      return result;
    } catch (e) {
      console.error("Failed to cleanup:", e);
      return null;
    }
  };

  return {
    // State
    blocks,
    isLoading,
    isEnhancing,
    contentBuffer,
    currentContext,

    // Actions
    loadTimeline,
    trackContent,
    createSnapshot,
    addManualEntry,
    editBlock,
    removeBlock,
    pinBlock,
    enhanceBlockSummary,
    runCleanup,
  };
}

// ==================== Content Trackers ====================

/**
 * Track document writing - captures actual content being written
 */
export function createDocumentTracker(
  getDocId: () => string | null,
  getDocTitle: () => string | null
) {
  const { trackContent } = useWorkStream();

  let lastContent = "";
  let debounceTimer: number | null = null;

  return {
    // Called with the actual content snippet being written
    trackWrite: (content: string) => {
      const id = getDocId();
      if (!id || !content || content === lastContent) return;

      // Debounce - only track after user pauses typing
      if (debounceTimer) clearTimeout(debounceTimer);

      debounceTimer = window.setTimeout(() => {
        lastContent = content;
        trackContent({
          type: "write",
          contextType: "document",
          contextId: id,
          contextTitle: getDocTitle() || undefined,
          content: content.slice(0, 200), // Keep snippets reasonable
        });
      }, 2000); // 2 second debounce
    },

    trackSave: (content?: string) => {
      const id = getDocId();
      if (!id) return;
      trackContent({
        type: "save",
        contextType: "document",
        contextId: id,
        contextTitle: getDocTitle() || undefined,
        content: content ? `Saved: ${content.slice(0, 100)}` : "Saved document",
      });
    },

    // Legacy compatibility - map to trackWrite
    trackEdit: (delta?: number, snippet?: string) => {
      if (snippet) {
        const id = getDocId();
        if (!id) return;
        trackContent({
          type: "write",
          contextType: "document",
          contextId: id,
          contextTitle: getDocTitle() || undefined,
          content: snippet.slice(0, 200),
        });
      }
    },

    // These are not valuable, keep as no-ops for compatibility
    trackFocus: () => {},
    trackBlur: () => {},
  };
}

/**
 * Track research/browsing - captures what was learned
 */
export function createBrowserTracker() {
  const { trackContent } = useWorkStream();

  return {
    trackResearch: (url: string, title: string, summary?: string) => {
      trackContent({
        type: "research",
        contextType: "browser",
        url,
        contextTitle: title,
        content: summary || title,
      });
    },

    trackSearch: (query: string, results?: string) => {
      trackContent({
        type: "research",
        contextType: "browser",
        contextTitle: `Search: ${query}`,
        content: results || query,
      });
    },
  };
}

/**
 * Track tool usage - captures what tool did
 */
export function createToolTracker() {
  const { trackContent } = useWorkStream();

  return {
    trackTool: (toolName: string, result?: string) => {
      trackContent({
        type: "tool",
        contextType: "mixed",
        content: result ? `${toolName}: ${result.slice(0, 100)}` : toolName,
      });
    },

    // Legacy compatibility
    trackToolStart: (_toolName: string, _input?: unknown) => {
      // Don't track starts - only track results
    },

    trackToolEnd: (toolName: string, result?: string, _success?: boolean) => {
      if (result) {
        trackContent({
          type: "tool",
          contextType: "mixed",
          content: `${toolName}: ${result.slice(0, 100)}`,
        });
      }
    },
  };
}
