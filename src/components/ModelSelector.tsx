import { Component, For, Show, createSignal, createMemo, onMount } from "solid-js";
import { AVAILABLE_MODELS, PROVIDER_PRESETS, ProviderConfig } from "../stores/settings";
import "./ModelSelector.css";

// Ollama model info interface
interface OllamaModel {
  name: string;
  size: number;  // bytes
  modified_at: string;
  digest: string;
}

// Format file size
function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

// Format time
function formatTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays} days ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
  return `${Math.floor(diffDays / 30)} months ago`;
}

// Provider type
type ProviderType = "cloud" | "ollama" | "custom";

// Ollama service status
type OllamaStatus = "checking" | "running" | "not-running";

interface ModelSelectorProps {
  value: string;
  onChange: (modelId: string) => void;
}

const ModelSelector: Component<ModelSelectorProps> = (props) => {
  // Determine current model's provider type
  const getCurrentProviderType = (): ProviderType => {
    const model = AVAILABLE_MODELS.find(m => m.id === props.value);
    if (!model) {
      // Check if it's an Ollama model (non-preset)
      if (props.value && !props.value.includes("/")) {
        return "ollama";
      }
      return "cloud";
    }
    if (model.provider === "ollama") return "ollama";
    if (model.provider === "custom") return "custom";
    return "cloud";
  };

  const [providerType, setProviderType] = createSignal<ProviderType>(getCurrentProviderType());
  const [ollamaStatus, setOllamaStatus] = createSignal<OllamaStatus>("checking");
  const [ollamaModels, setOllamaModels] = createSignal<OllamaModel[]>([]);
  const [ollamaBaseUrl, _setOllamaBaseUrl] = createSignal("http://localhost:11434");

  // Cloud provider categories
  const cloudProviderCategories = {
    official: {
      name: "Official API",
      providers: ["anthropic", "openai", "google", "minimax", "deepseek"]
    },
    aggregator: {
      name: "Aggregation Services",
      providers: ["openrouter", "together", "groq", "siliconflow"]
    }
  };

  // Get cloud models grouped by provider
  const cloudModels = createMemo(() => {
    const result: Record<string, { name: string; models: typeof AVAILABLE_MODELS }> = {};

    for (const [key, category] of Object.entries(cloudProviderCategories)) {
      const models = AVAILABLE_MODELS.filter(m => category.providers.includes(m.provider));
      if (models.length > 0) {
        result[key] = { name: category.name, models };
      }
    }
    return result;
  });

  // Check Ollama service status and get model list
  const checkOllamaStatus = async () => {
    setOllamaStatus("checking");
    try {
      const baseUrl = ollamaBaseUrl().replace(/\/$/, "");
      const response = await fetch(`${baseUrl}/api/tags`, {
        method: "GET",
        signal: AbortSignal.timeout(5000),
      });

      if (response.ok) {
        const data = await response.json();
        const models = data.models || [];
        setOllamaModels(models);
        setOllamaStatus("running");
      } else {
        setOllamaStatus("not-running");
        setOllamaModels([]);
      }
    } catch {
      setOllamaStatus("not-running");
      setOllamaModels([]);
    }
  };

  // Check Ollama status on mount
  onMount(() => {
    checkOllamaStatus();
  });

  // Currently selected Ollama model
  const selectedOllamaModel = createMemo(() => {
    return ollamaModels().find(m => m.name === props.value);
  });

  // Handle model selection (baseUrl is auto-updated in settings store)
  const handleCloudModelChange = (modelId: string) => {
    props.onChange(modelId);
  };

  const handleOllamaModelSelect = (modelName: string) => {
    props.onChange(modelName);
  };

  // Get current provider info
  const currentProviderInfo = createMemo((): ProviderConfig | null => {
    const model = AVAILABLE_MODELS.find(m => m.id === props.value);
    if (!model) return null;
    return PROVIDER_PRESETS[model.provider] || null;
  });

  return (
    <div class="model-selector">
      {/* Provider type tabs */}
      <div class="provider-tabs">
        <button
          class={`provider-tab ${providerType() === "cloud" ? "active" : ""}`}
          onClick={() => setProviderType("cloud")}
        >
          <span class="tab-icon">☁️</span>
          <span class="tab-label">Cloud</span>
        </button>
        <button
          class={`provider-tab ${providerType() === "ollama" ? "active" : ""}`}
          onClick={() => {
            setProviderType("ollama");
            checkOllamaStatus();
          }}
        >
          <span class="tab-icon">🦙</span>
          <span class="tab-label">Ollama</span>
        </button>
        <button
          class={`provider-tab ${providerType() === "custom" ? "active" : ""}`}
          onClick={() => setProviderType("custom")}
        >
          <span class="tab-icon">⚙️</span>
          <span class="tab-label">Custom</span>
        </button>
      </div>

      {/* Cloud service selection */}
      <Show when={providerType() === "cloud"}>
        <div class="cloud-selector">
          <select
            value={props.value}
            onChange={(e) => handleCloudModelChange(e.currentTarget.value)}
          >
            <For each={Object.entries(cloudModels())}>
              {([_key, categoryData]) => (
                <optgroup label={categoryData.name}>
                  <For each={categoryData.models}>
                    {(model) => (
                      <option value={model.id}>
                        {model.name} - {model.description}
                      </option>
                    )}
                  </For>
                </optgroup>
              )}
            </For>
          </select>

          <Show when={currentProviderInfo()}>
            <div class="selected-info">
              <span class="info-badge">{currentProviderInfo()?.name}</span>
              <span class="info-desc">{currentProviderInfo()?.description}</span>
            </div>
          </Show>
        </div>
      </Show>

      {/* Ollama model selection */}
      <Show when={providerType() === "ollama"}>
        <div class="ollama-section">
          {/* Status indicator */}
          <Show when={ollamaStatus() === "checking"}>
            <div class="ollama-status checking">
              <span class="status-icon">⏳</span>
              <p>Checking Ollama service...</p>
            </div>
          </Show>

          <Show when={ollamaStatus() === "not-running"}>
            <div class="ollama-status not-running">
              <span class="status-icon">⚠️</span>
              <div class="status-content">
                <p><strong>Ollama not running</strong></p>
                <p>Please install and start <a href="https://ollama.ai" target="_blank">Ollama</a></p>
                <button class="retry-btn" onClick={checkOllamaStatus}>
                  Retry
                </button>
              </div>
            </div>
          </Show>

          <Show when={ollamaStatus() === "running"}>
            <div class="ollama-status running">
              <span class="status-icon">✅</span>
              <p>Ollama running · {ollamaModels().length} models</p>
              <button class="refresh-btn" onClick={checkOllamaStatus}>
                Refresh
              </button>
            </div>

            <Show when={ollamaModels().length === 0}>
              <div class="no-models">
                <p>No models installed</p>
                <p class="hint">Run <code>ollama pull llama3.2</code> to install a model</p>
              </div>
            </Show>

            <Show when={ollamaModels().length > 0}>
              <div class="model-list">
                <For each={ollamaModels()}>
                  {(model) => (
                    <div
                      class={`model-item ${selectedOllamaModel()?.name === model.name ? "selected" : ""}`}
                      onClick={() => handleOllamaModelSelect(model.name)}
                    >
                      <div class="model-main">
                        <span class="model-name">{model.name}</span>
                        <span class="model-size">{formatSize(model.size)}</span>
                      </div>
                      <div class="model-meta">
                        <span class="model-time">Updated {formatTime(model.modified_at)}</span>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </Show>

          {/* Current selected model */}
          <Show when={selectedOllamaModel()}>
            <div class="selected-model-info">
              <span class="label">Current model:</span>
              <span class="value">{selectedOllamaModel()?.name}</span>
            </div>
          </Show>
        </div>
      </Show>

      {/* Custom service */}
      <Show when={providerType() === "custom"}>
        <div class="custom-section">
          <div class="custom-notice">
            <span class="notice-icon">🔧</span>
            <p>Use OpenAI-compatible API service (vLLM / TGI / SGLang, etc.)</p>
          </div>

          <div class="custom-form">
            <div class="form-group">
              <label>Model ID</label>
              <input
                type="text"
                value={props.value === "custom-model" ? "" : props.value}
                placeholder="e.g., meta-llama/Llama-3.2-8B"
                onInput={(e) => props.onChange(e.currentTarget.value || "custom-model")}
              />
            </div>
            <p class="hint">Configure API URL and key in Settings</p>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default ModelSelector;
