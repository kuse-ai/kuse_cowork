import { createSignal } from "solid-js";
import {
  Trace,
  TraceInput,
  TraceSettings,
  Suggestion,
  logTrace as logTraceApi,
  listTraces as listTracesApi,
  deleteTrace as deleteTraceApi,
  clearTraces as clearTracesApi,
  getTraceSettings as getTraceSettingsApi,
  saveTraceSettings as saveTraceSettingsApi,
  listSuggestions as listSuggestionsApi,
  updateSuggestionStatus as updateSuggestionStatusApi,
  deleteSuggestion as deleteSuggestionApi,
  generateSuggestions as generateSuggestionsApi,
  applySuggestion as applySuggestionApi,
} from "../lib/tauri-api";

export type { Trace, TraceInput, TraceSettings, Suggestion };

// Trace state
const [traces, setTraces] = createSignal<Trace[]>([]);
const [traceSettings, setTraceSettings] = createSignal<TraceSettings>({
  doc_id: null,
  tracing_enabled: true,
  include_snippets: true,
});
const [isLoadingTraces, setIsLoadingTraces] = createSignal(false);
const [currentDocId, setCurrentDocId] = createSignal<string | null>(null);

// Suggestion state
const [suggestions, setSuggestions] = createSignal<Suggestion[]>([]);
const [isLoadingSuggestions, setIsLoadingSuggestions] = createSignal(false);

export function useTraces() {
  const loadTraces = async (docId: string, limit?: number) => {
    setIsLoadingTraces(true);
    setCurrentDocId(docId);
    try {
      const traceList = await listTracesApi(docId, limit);
      setTraces(traceList);
      const settings = await getTraceSettingsApi(docId);
      setTraceSettings(settings);
    } catch (e) {
      console.error("Failed to load traces:", e);
    } finally {
      setIsLoadingTraces(false);
    }
  };

  const logTrace = async (input: TraceInput) => {
    // Check if tracing is enabled
    if (!traceSettings().tracing_enabled) {
      return null;
    }
    try {
      const trace = await logTraceApi(input);
      // Add to local state if this is for the current doc
      if (input.doc_id === currentDocId()) {
        setTraces((prev) => [trace, ...prev]);
      }
      return trace;
    } catch (e) {
      console.error("Failed to log trace:", e);
      return null;
    }
  };

  const deleteTrace = async (id: string) => {
    try {
      await deleteTraceApi(id);
      setTraces((prev) => prev.filter((t) => t.id !== id));
    } catch (e) {
      console.error("Failed to delete trace:", e);
    }
  };

  const clearTraces = async (docId: string) => {
    try {
      await clearTracesApi(docId);
      if (docId === currentDocId()) {
        setTraces([]);
      }
    } catch (e) {
      console.error("Failed to clear traces:", e);
    }
  };

  const updateSettings = async (updates: Partial<TraceSettings>) => {
    const docId = currentDocId();
    if (!docId) return;

    const newSettings = { ...traceSettings(), ...updates };
    try {
      await saveTraceSettingsApi(docId, newSettings);
      setTraceSettings(newSettings);
    } catch (e) {
      console.error("Failed to update trace settings:", e);
    }
  };

  const loadMoreTraces = async () => {
    const docId = currentDocId();
    if (!docId) return;

    const currentTraces = traces();
    if (currentTraces.length === 0) return;

    const oldestTimestamp = currentTraces[currentTraces.length - 1].created_at;
    try {
      const moreTraces = await listTracesApi(docId, 50, oldestTimestamp);
      setTraces((prev) => [...prev, ...moreTraces]);
    } catch (e) {
      console.error("Failed to load more traces:", e);
    }
  };

  return {
    traces,
    traceSettings,
    isLoadingTraces,
    currentDocId,
    loadTraces,
    logTrace,
    deleteTrace,
    clearTraces,
    updateSettings,
    loadMoreTraces,
  };
}

// Additional state for suggestion generation
const [isGeneratingSuggestions, setIsGeneratingSuggestions] = createSignal(false);
const [isApplyingSuggestion, setIsApplyingSuggestion] = createSignal(false);

export function useSuggestions() {
  const loadSuggestions = async (docId: string, status?: string) => {
    setIsLoadingSuggestions(true);
    try {
      const suggestionList = await listSuggestionsApi(docId, status);
      setSuggestions(suggestionList);
    } catch (e) {
      console.error("Failed to load suggestions:", e);
    } finally {
      setIsLoadingSuggestions(false);
    }
  };

  const generateSuggestions = async (docId: string) => {
    setIsGeneratingSuggestions(true);
    try {
      const newSuggestions = await generateSuggestionsApi(docId);
      setSuggestions((prev) => [...newSuggestions, ...prev]);
      return newSuggestions;
    } catch (e) {
      console.error("Failed to generate suggestions:", e);
      return [];
    } finally {
      setIsGeneratingSuggestions(false);
    }
  };

  const approveSuggestion = async (id: string) => {
    try {
      await updateSuggestionStatusApi(id, "approved");
      setSuggestions((prev) =>
        prev.map((s) => (s.id === id ? { ...s, status: "approved" } : s))
      );
    } catch (e) {
      console.error("Failed to approve suggestion:", e);
    }
  };

  const applySuggestion = async (id: string) => {
    setIsApplyingSuggestion(true);
    try {
      const result = await applySuggestionApi(id);
      setSuggestions((prev) =>
        prev.map((s) => (s.id === id ? { ...s, status: "approved" } : s))
      );
      return result;
    } catch (e) {
      console.error("Failed to apply suggestion:", e);
      return null;
    } finally {
      setIsApplyingSuggestion(false);
    }
  };

  const rejectSuggestion = async (id: string) => {
    try {
      await updateSuggestionStatusApi(id, "rejected");
      setSuggestions((prev) =>
        prev.map((s) => (s.id === id ? { ...s, status: "rejected" } : s))
      );
    } catch (e) {
      console.error("Failed to reject suggestion:", e);
    }
  };

  const dismissSuggestion = async (id: string) => {
    try {
      await deleteSuggestionApi(id);
      setSuggestions((prev) => prev.filter((s) => s.id !== id));
    } catch (e) {
      console.error("Failed to dismiss suggestion:", e);
    }
  };

  return {
    suggestions,
    isLoadingSuggestions,
    isGeneratingSuggestions,
    isApplyingSuggestion,
    loadSuggestions,
    generateSuggestions,
    approveSuggestion,
    applySuggestion,
    rejectSuggestion,
    dismissSuggestion,
  };
}
