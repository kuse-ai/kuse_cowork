import { Component, createSignal, onCleanup, onMount } from "solid-js";
import { useTraces } from "../stores/traces";
import {
  createEmbeddedBrowser,
  updateEmbeddedBrowserBounds,
  navigateEmbeddedBrowser,
  closeEmbeddedBrowser,
  openBrowserWindow,
  isTauri,
} from "../lib/tauri-api";
import "./BrowserPanel.css";

interface BrowserPanelProps {
  docId: string | null;
  onClose?: () => void;
}

const Icons = {
  Refresh: () => (
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
      <path d="M3 3v5h5" />
      <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
      <path d="M16 21h5v-5" />
    </svg>
  ),
  ExternalLink: () => (
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
      <polyline points="15 3 21 3 21 9" />
      <line x1="10" y1="14" x2="21" y2="3" />
    </svg>
  ),
  Close: () => (
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  ),
  ArrowRight: () => (
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <line x1="5" y1="12" x2="19" y2="12" />
      <polyline points="12 5 19 12 12 19" />
    </svg>
  ),
  Search: () => (
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
  ),
  Globe: () => (
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  )
};

const BrowserPanel: Component<BrowserPanelProps> = (props) => {
  const [inputUrl, setInputUrl] = createSignal("https://www.google.com");
  const [currentUrl, setCurrentUrl] = createSignal("");
  const [isLoading, setIsLoading] = createSignal(false);
  const [browserActive, setBrowserActive] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [isCreatingBrowser, setIsCreatingBrowser] = createSignal(false);

  const { logTrace } = useTraces();
  let containerRef: HTMLDivElement | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let resizeTimeout: number | undefined;
  let ticking = false;

  // Log URL visit as browse trace
  const logBrowseTrace = (navigatedUrl: string) => {
    if (props.docId) {
      logTrace({
        doc_id: props.docId,
        event_type: "browse",
        payload: { url: navigatedUrl },
      });
    }
  };

  const getBounds = () => {
    if (!containerRef) return null;

    // Get the panel (parent) element and toolbar
    const panelElement = containerRef.parentElement;
    const toolbarElement = panelElement?.querySelector('.browser-toolbar') as HTMLElement;
    const statusBarElement = panelElement?.querySelector('.browser-status-bar') as HTMLElement;

    if (!panelElement || !toolbarElement) {
      console.log("Panel or toolbar not found");
      return null;
    }

    const panelRect = panelElement.getBoundingClientRect();
    const toolbarRect = toolbarElement.getBoundingClientRect();
    const statusBarHeight = statusBarElement?.offsetHeight || 0;

    // Calculate webview bounds based on actual element positions
    // Use toolbar.bottom directly to avoid offset calculation errors
    // Add 2px visual gap + 30px title bar compensation
    // The native webview appears to position relative to the window frame (including title bar)
    // while getBoundingClientRect is relative to the content view.
    const titleBarOffset = 30;
    let y = toolbarRect.bottom + titleBarOffset + 2;
    
    // Safety guard: Ensure y is at least 40px + offset from panel top
    const minY = panelRect.top + 40 + titleBarOffset;
    if (y < minY) {
      console.warn("Toolbar rect seems too small, enforcing minimum offset");
      y = minY;
    }

    const x = panelRect.left + 1;
    const width = Math.max(0, panelRect.width - 2);
    // Height is from computed Y to panel bottom (minus status bar)
    const height = Math.max(0, panelRect.bottom - y + titleBarOffset - statusBarHeight);

    console.log("Panel rect:", panelRect);
    console.log("Toolbar rect:", toolbarRect, "Status bar height:", statusBarHeight);
    console.log("Calculated webview bounds:", { x, y, width, height });

    return { x, y, width, height };
  };

  const updateBounds = async () => {
    // Don't update bounds if browser isn't active or if we're in the middle of creating it
    if (!browserActive() || isCreatingBrowser()) return;

    const bounds = getBounds();
    // Sanity check: bounds should be reasonable
    if (bounds && bounds.width > 20 && bounds.height > 20 && bounds.y > 0) {
      try {
        console.log("Updating webview bounds to:", bounds);
        await updateEmbeddedBrowserBounds(bounds.x, bounds.y, bounds.width, bounds.height);
      } catch (e) {
        console.error("Failed to update browser bounds:", e);
      }
    } else {
      console.log("Skipping bounds update - invalid bounds:", bounds);
    }
  };

  const debouncedUpdateBounds = () => {
    if (!ticking) {
      window.requestAnimationFrame(async () => {
        await updateBounds();
        ticking = false;
      });
      ticking = true;
    }
  };

  // Wait for layout to stabilize
  const waitForLayout = (ms: number = 50): Promise<void> => {
    return new Promise((resolve) => {
      requestAnimationFrame(() => {
        setTimeout(() => {
          requestAnimationFrame(() => {
            resolve();
          });
        }, ms);
      });
    });
  };

  const openEmbeddedBrowser = async (url: string) => {
    if (!isTauri()) {
      setError("Embedded browser requires the desktop app");
      return;
    }

    let normalizedUrl = url.trim();
    if (!normalizedUrl.startsWith("http://") && !normalizedUrl.startsWith("https://")) {
      normalizedUrl = "https://" + normalizedUrl;
    }

    setIsLoading(true);
    setError(null);

    try {
      // Wait for layout to stabilize before getting bounds
      await waitForLayout(100);

      let bounds = getBounds();
      console.log("Initial browser bounds:", bounds);

      // If container doesn't seem ready, wait a bit more
      if (!bounds || bounds.width <= 0 || bounds.height <= 100) {
        console.log("Bounds seem incorrect, waiting longer...");
        await waitForLayout(200);
        bounds = getBounds();
        console.log("Retried browser bounds:", bounds);
      }

      if (!bounds || bounds.width <= 0 || bounds.height <= 0) {
        throw new Error("Browser container not ready");
      }

      // Sanity check: y should be reasonable (below toolbar, around 40-100px from panel top)
      console.log("Final bounds for webview:", bounds);

      if (browserActive()) {
        // Just navigate if already active
        console.log("Navigating existing browser to:", normalizedUrl);
        await navigateEmbeddedBrowser(normalizedUrl);
        // Force update bounds after navigation in case the site tried to resize the window
        setTimeout(updateBounds, 100);
        setTimeout(updateBounds, 500);
      } else {
        // Create new embedded browser
        setIsCreatingBrowser(true);
        try {
          console.log("Creating embedded browser at:", bounds, "URL:", normalizedUrl);
          const result = await createEmbeddedBrowser(
            normalizedUrl,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height
          );
          console.log("Embedded browser created:", result);
          setBrowserActive(true);

          // Wait a bit before allowing bounds updates to prevent race conditions
          await waitForLayout(500);
        } finally {
          setIsCreatingBrowser(false);
        }
      }

      setCurrentUrl(normalizedUrl);
      setInputUrl(normalizedUrl);
      logBrowseTrace(normalizedUrl);
    } catch (e) {
      console.error("Failed to open embedded browser:", e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const handleNavigate = () => {
    if (inputUrl().trim()) {
      openEmbeddedBrowser(inputUrl());
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleNavigate();
    }
  };

  const handleRefresh = () => {
    if (currentUrl()) {
      openEmbeddedBrowser(currentUrl());
    }
  };

  const handleOpenExternal = async () => {
    const url = currentUrl() || inputUrl();
    if (!url.trim()) return;

    let normalizedUrl = url;
    if (!normalizedUrl.startsWith("http://") && !normalizedUrl.startsWith("https://")) {
      normalizedUrl = "https://" + normalizedUrl;
    }

    try {
      await openBrowserWindow(normalizedUrl, "External Browser");
    } catch {
      window.open(normalizedUrl, "_blank");
    }
  };

  const handleClose = async () => {
    if (browserActive()) {
      try {
        await closeEmbeddedBrowser();
      } catch (e) {
        console.error("Failed to close embedded browser:", e);
      }
      setBrowserActive(false);
    }
    props.onClose?.();
  };

  // Set up resize observer with optimized throttling for 60fps updates
  onMount(() => {
    if (containerRef) {
      resizeObserver = new ResizeObserver(() => {
        debouncedUpdateBounds();
      });
      resizeObserver.observe(containerRef);

      // Listen for transition end (for panel sliding animations)
      containerRef.addEventListener("transitionend", debouncedUpdateBounds);

      // Also listen for window resize and scroll
      window.addEventListener("resize", debouncedUpdateBounds);
      window.addEventListener("scroll", debouncedUpdateBounds, true);

      // Initial bounds update after a small delay to ensure layout is stable
      setTimeout(updateBounds, 100);
    }
  });

  // Clean up on unmount
  onCleanup(async () => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
    if (containerRef) {
      containerRef.removeEventListener("transitionend", debouncedUpdateBounds);
    }
    
    // Remove global listeners
    window.removeEventListener("resize", debouncedUpdateBounds);
    window.removeEventListener("scroll", debouncedUpdateBounds, true);

    if (resizeTimeout) {
      clearTimeout(resizeTimeout);
    }
    
    // Close the embedded browser when panel unmounts
    if (browserActive()) {
      try {
        await closeEmbeddedBrowser();
      } catch (e) {
        console.error("Failed to close embedded browser on cleanup:", e);
      }
    }
  });

  return (
    <div class="browser-panel">
      <div class="browser-toolbar">
        <div class="browser-nav-buttons">
          <button
            class="browser-nav-btn"
            onClick={handleRefresh}
            disabled={!currentUrl() || isLoading()}
            title="Refresh"
          >
            <Icons.Refresh />
          </button>
        </div>
        <div class="browser-url-bar">
          <div class="browser-input-wrapper">
            <span class="browser-input-icon">
              {currentUrl() ? <Icons.Globe /> : <Icons.Search />}
            </span>
            <input
              type="text"
              value={inputUrl()}
              onInput={(e) => setInputUrl(e.currentTarget.value)}
              onKeyDown={handleKeyDown}
              placeholder="Search or enter website name..."
            />
          </div>
          <button
            class="browser-go-btn"
            onClick={handleNavigate}
            disabled={isLoading()}
            title="Go"
          >
            <Icons.ArrowRight />
          </button>
        </div>
        <div class="browser-actions">
          <button
            class="browser-action-btn"
            onClick={handleOpenExternal}
            title="Open in separate window"
          >
            <Icons.ExternalLink />
          </button>
          <button 
            class="browser-action-btn close" 
            onClick={handleClose}
            title="Close browser panel"
          >
            <Icons.Close />
          </button>
        </div>
      </div>

      {error() && (
        <div class="browser-error">
          {error()}
        </div>
      )}

      <div
        ref={containerRef}
        class="browser-webview-container"
      >
        {!browserActive() && !isLoading() && (
          <div class="browser-placeholder">
            <p>Enter a URL and click Go to browse</p>
            <p class="hint">The browser opens as a native webview overlay</p>
          </div>
        )}
        {isLoading() && (
          <div class="browser-loading">
            <div class="browser-loading-spinner" />
            <p>Loading...</p>
          </div>
        )}
      </div>

      <div class="browser-status-bar">
        <span class="browser-status-url">{currentUrl() || "No page loaded"}</span>
      </div>
    </div>
  );
};

export default BrowserPanel;
