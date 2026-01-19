import { Component, createSignal, createMemo, Show } from "solid-js";
import { useSettings, AVAILABLE_MODELS, PROVIDER_PRESETS } from "../stores/settings";
import { testConnection } from "../lib/tauri-api";
import ModelSelector from "./ModelSelector";
import "./Settings.css";

const Settings: Component = () => {
  const { settings, updateSetting, toggleSettings } = useSettings();
  const [testing, setTesting] = createSignal(false);
  const [testResult, setTestResult] = createSignal<string | null>(null);


  // Get current selected model's provider info
  const currentProviderInfo = createMemo(() => {
    const model = AVAILABLE_MODELS.find(m => m.id === settings().model);
    if (model) {
      return PROVIDER_PRESETS[model.provider];
    }
    // If not in preset list, check baseUrl to determine provider
    const baseUrl = settings().baseUrl;
    if (baseUrl.includes("localhost:11434") || baseUrl.includes("127.0.0.1:11434")) {
      return PROVIDER_PRESETS["ollama"];
    }
    if (baseUrl.includes("localhost:8080")) {
      return PROVIDER_PRESETS["localai"];
    }
    // Other local services
    if (baseUrl.includes("localhost") || baseUrl.includes("127.0.0.1")) {
      return PROVIDER_PRESETS["custom"];
    }
    return null;
  });

  // Check if it's a local service (no API Key needed)
  const isLocalProvider = createMemo(() => {
    const info = currentProviderInfo();
    if (info?.authType === "none") return true;
    // Additional check: if baseUrl is a local address, treat as local service
    const baseUrl = settings().baseUrl;
    if (baseUrl.includes("localhost") || baseUrl.includes("127.0.0.1")) {
      return true;
    }
    return false;
  });

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      console.log("Testing connection...");
      const result = await testConnection();
      console.log("Test result:", result, typeof result);
      setTestResult(result);
    } catch (e) {
      console.error("Test connection error:", e);
      const errorMsg = e instanceof Error ? e.message : String(e);
      setTestResult(`Error: ${errorMsg}`);
    }
    setTesting(false);
  };

  // const handleSave = async () => {
  //   setSaving(true);
  //   await saveAllSettings(settings());
  //   setSaving(false);
  // };

  return (
    <div class="settings">
      <div class="settings-header">
        <h2>Settings</h2>
        <button class="close-btn" onClick={toggleSettings}>
          Close
        </button>
      </div>

      <div class="settings-content">
        <div class="settings-section">
          <h3>Model Selection</h3>

          <ModelSelector
            value={settings().model}
            onChange={(modelId, baseUrl) => {
              updateSetting("model", modelId);
              if (baseUrl) {
                updateSetting("baseUrl", baseUrl);
              }
            }}
          />
        </div>

        <div class="settings-section">
          <h3>API Configuration</h3>

          {/* Local service notice */}
          <Show when={isLocalProvider()}>
            <div class="local-service-notice">
              <span class="notice-icon">🏠</span>
              <div class="notice-content">
                <strong>Local Service - No API Key Required</strong>
                <p>Please ensure {currentProviderInfo()?.name} is running locally</p>
              </div>
            </div>
          </Show>

          {/* API Key input - only show when needed */}
          <Show when={!isLocalProvider()}>
            <div class="form-group">
              <label for="apiKey">API Key</label>
              <input
                id="apiKey"
                type="password"
                value={settings().apiKey}
                onInput={(e) => updateSetting("apiKey", e.currentTarget.value)}
                placeholder={currentProviderInfo()?.authType === "bearer" ? "sk-..." : "your-api-key"}
              />
              <span class="hint">
                <Show
                  when={currentProviderInfo()?.id === "anthropic"}
                  fallback={<>Get API Key from {currentProviderInfo()?.name}</>}
                >
                  Get your API key from{" "}
                  <a href="https://console.anthropic.com/settings/keys" target="_blank">
                    Anthropic Console
                  </a>
                </Show>
              </span>
            </div>
          </Show>

          <div class="form-group">
            <label for="baseUrl">API Base URL</label>
            <input
              id="baseUrl"
              type="text"
              value={settings().baseUrl}
              onInput={(e) => updateSetting("baseUrl", e.currentTarget.value)}
              placeholder={currentProviderInfo()?.baseUrl || "https://api.example.com"}
            />
            <span class="hint">
              {isLocalProvider()
                ? "Ensure the address matches your local service configuration"
                : "Customize proxy or compatible API address"}
            </span>
          </div>

          <div class="form-group">
            <label for="maxTokens">Max Tokens</label>
            <input
              id="maxTokens"
              type="number"
              value={settings().maxTokens}
              onInput={(e) =>
                updateSetting("maxTokens", parseInt(e.currentTarget.value) || 4096)
              }
              min={1}
              max={200000}
            />
          </div>

          <div class="form-group">
            <button
              class="test-btn"
              onClick={handleTest}
              disabled={testing() || (!isLocalProvider() && !settings().apiKey)}
            >
              {testing() ? "Testing..." : "Test Connection"}
            </button>
            {testResult() === "success" && (
              <span class="test-success">✓ Connection successful!</span>
            )}
            {testResult() && testResult() !== "success" && (
              <span class="test-error">{testResult()}</span>
            )}
          </div>
        </div>

        <div class="settings-section">
          <h3>Data Storage</h3>
          <p class="hint" style={{ margin: 0 }}>
            All data is stored locally on your computer in SQLite database.
            <br />
            API key is securely stored and never sent to any server except Anthropic's API.
          </p>
        </div>
      </div>
    </div>
  );
};

export default Settings;
