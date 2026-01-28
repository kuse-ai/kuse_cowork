import { createSignal, onCleanup, onMount } from "solid-js";
import {
  CaptureConfig,
  SourceLinkWithSources,
  BatchInsertResult,
  reportPageContext,
  updatePageContext,
  captureSearch,
  updateSearchClick,
  captureAIExchange,
  flushCaptureBuffer,
  activateSource,
  deactivateSource,
  createSourceLink,
  getDocumentProvenance,
  getCaptureConfig,
  updateCaptureConfig,
  detectSearchQuery,
  startFlushTimer,
  stopFlushTimer,
} from "../lib/capture-api";

export type { CaptureConfig, SourceLinkWithSources, BatchInsertResult };

// ==================== State ====================

const [config, setConfig] = createSignal<CaptureConfig>({
  clipboard_enabled: true,
  browse_enabled: true,
  search_enabled: true,
  ai_exchange_enabled: true,
  source_linking_enabled: true,
  flush_interval_secs: 30,
  clipboard_poll_ms: 500,
});

const [isInitialized, setIsInitialized] = createSignal(false);
const [lastFlush, setLastFlush] = createSignal<BatchInsertResult | null>(null);

// Current page tracking
const [currentBrowseId, setCurrentBrowseId] = createSignal<string | null>(null);
const [currentSearchId, setCurrentSearchId] = createSignal<string | null>(null);

// ==================== Main Hook ====================

export function useCapture() {
  onMount(async () => {
    if (!isInitialized()) {
      try {
        const loadedConfig = await getCaptureConfig();
        setConfig(loadedConfig);
        startFlushTimer(loadedConfig.flush_interval_secs);
        setIsInitialized(true);
      } catch (e) {
        console.error("Failed to initialize capture:", e);
      }
    }
  });

  onCleanup(() => {
    stopFlushTimer();
  });

  // ==================== Page Context ====================

  /**
   * Track entering a page (called from BrowserPanel)
   */
  const trackPageEnter = async (url: string, title: string | null) => {
    console.log("[Capture] trackPageEnter called:", { url, title, browseEnabled: config().browse_enabled });
    if (!config().browse_enabled) {
      console.log("[Capture] Browse capture disabled, skipping");
      return;
    }

    // Close previous page if any
    if (currentBrowseId()) {
      console.log("[Capture] Closing previous page:", currentBrowseId());
      await updatePageContext(currentBrowseId()!);
    }

    try {
      console.log("[Capture] Reporting page context...");
      const browseId = await reportPageContext(url, title);
      console.log("[Capture] Page context reported, browseId:", browseId);
      setCurrentBrowseId(browseId);

      // Activate as a source for linking
      if (config().source_linking_enabled) {
        await activateSource("webpage", browseId, title ?? undefined);
      }

      // Check for search query
      if (config().search_enabled) {
        const search = detectSearchQuery(url);
        if (search) {
          console.log("[Capture] Detected search:", search);
          const searchId = await captureSearch(search.query, search.engine);
          setCurrentSearchId(searchId);
        }
      }
    } catch (e) {
      console.error("[Capture] Failed to track page enter:", e);
    }
  };

  /**
   * Track leaving a page (called from BrowserPanel on URL change)
   */
  const trackPageLeave = async (scrollDepthPercent?: number) => {
    if (!currentBrowseId()) return;

    try {
      await updatePageContext(currentBrowseId()!, Date.now(), scrollDepthPercent);

      // Deactivate the source
      if (config().source_linking_enabled) {
        await deactivateSource(currentBrowseId()!);
      }

      setCurrentBrowseId(null);
    } catch (e) {
      console.error("Failed to track page leave:", e);
    }
  };

  /**
   * Track clicking a search result
   */
  const trackSearchClick = async (clickedUrl: string) => {
    if (!currentSearchId()) return;

    try {
      await updateSearchClick(currentSearchId()!, clickedUrl);
      setCurrentSearchId(null);
    } catch (e) {
      console.error("Failed to track search click:", e);
    }
  };

  // ==================== AI Exchange ====================

  /**
   * Capture an AI exchange (called after receiving AI response)
   */
  const trackAIExchange = async (
    question: string,
    answer: string,
    model: string,
    contextDocId?: string
  ) => {
    if (!config().ai_exchange_enabled) return;

    try {
      await captureAIExchange(question, answer, model, contextDocId);
    } catch (e) {
      console.error("Failed to track AI exchange:", e);
    }
  };

  // ==================== Source Linking ====================

  /**
   * Create a source link when saving document content
   */
  const linkSourcesOnSave = async (
    docId: string,
    content: string,
    sectionPath?: string
  ): Promise<SourceLinkWithSources | null> => {
    if (!config().source_linking_enabled) return null;

    try {
      return await createSourceLink(docId, content, sectionPath);
    } catch (e) {
      console.error("Failed to create source link:", e);
      return null;
    }
  };

  /**
   * Get provenance for a document
   */
  const getProvenance = async (docId: string): Promise<SourceLinkWithSources[]> => {
    try {
      return await getDocumentProvenance(docId);
    } catch (e) {
      console.error("Failed to get provenance:", e);
      return [];
    }
  };

  // ==================== Flush ====================

  /**
   * Manually flush the capture buffer
   */
  const flush = async (): Promise<BatchInsertResult | null> => {
    try {
      const result = await flushCaptureBuffer();
      setLastFlush(result);
      return result;
    } catch (e) {
      console.error("Failed to flush capture buffer:", e);
      return null;
    }
  };

  // ==================== Config ====================

  /**
   * Update capture configuration
   */
  const updateConfig = async (newConfig: Partial<CaptureConfig>) => {
    const merged = { ...config(), ...newConfig };
    try {
      await updateCaptureConfig(merged);
      setConfig(merged);

      // Restart flush timer if interval changed
      if (newConfig.flush_interval_secs !== undefined) {
        stopFlushTimer();
        startFlushTimer(merged.flush_interval_secs);
      }
    } catch (e) {
      console.error("Failed to update config:", e);
    }
  };

  return {
    // State
    config,
    isInitialized,
    lastFlush,
    currentBrowseId,

    // Page tracking
    trackPageEnter,
    trackPageLeave,
    trackSearchClick,

    // AI exchange
    trackAIExchange,

    // Source linking
    linkSourcesOnSave,
    getProvenance,

    // Flush
    flush,

    // Config
    updateConfig,
  };
}

// ==================== Page Tracker Hook ====================

/**
 * Create a page tracker for BrowserPanel
 */
export function createPageTracker() {
  const {
    trackPageEnter,
    trackPageLeave,
    trackSearchClick,
    config,
  } = useCapture();

  let lastUrl: string | null = null;

  return {
    /**
     * Called when navigating to a new URL
     */
    onNavigate: async (url: string, title: string | null) => {
      // Skip if same URL
      if (url === lastUrl) return;

      // Leave previous page
      if (lastUrl) {
        await trackPageLeave();
      }

      // Enter new page
      lastUrl = url;
      await trackPageEnter(url, title);
    },

    /**
     * Called when clicking a link (could be search result)
     */
    onClick: async (clickedUrl: string) => {
      await trackSearchClick(clickedUrl);
    },

    /**
     * Called on scroll (for scroll depth tracking)
     */
    onScroll: (percent: number) => {
      // Could debounce and update periodically
      // For now, just track on leave
    },

    /**
     * Called when closing the browser panel
     */
    onClose: async () => {
      if (lastUrl) {
        await trackPageLeave();
        lastUrl = null;
      }
    },
  };
}

// ==================== AI Tracker Hook ====================

/**
 * Create an AI exchange tracker for chat components
 */
export function createAITracker(getContextDocId: () => string | null) {
  const { trackAIExchange, config } = useCapture();

  return {
    /**
     * Called after receiving an AI response
     */
    onResponse: async (question: string, answer: string, model: string) => {
      await trackAIExchange(question, answer, model, getContextDocId() ?? undefined);
    },
  };
}

// ==================== Source Linker Hook ====================

/**
 * Create a source linker for document save
 */
export function createSourceLinker() {
  const { linkSourcesOnSave, getProvenance, config } = useCapture();

  return {
    /**
     * Link sources when saving content
     */
    onSave: async (docId: string, content: string, sectionPath?: string) => {
      return await linkSourcesOnSave(docId, content, sectionPath);
    },

    /**
     * Get provenance for a document
     */
    getProvenance: async (docId: string) => {
      return await getProvenance(docId);
    },
  };
}
