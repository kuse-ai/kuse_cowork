import { createSignal } from "solid-js";
import {
  getSettings as getSettingsApi,
  saveSettings as saveSettingsApi,
  Settings as ApiSettings,
} from "../lib/tauri-api";

export interface Settings {
  apiKey: string;  // Current active API key (for display)
  model: string;
  baseUrl: string;
  maxTokens: number;
  temperature?: number;
  providerKeys: Record<string, string>;  // Provider-specific API keys
  openaiOrganization?: string;  // Optional OpenAI Organization ID
  openaiProject?: string;  // Optional OpenAI Project ID
}

// Provider configuration type
export interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  apiFormat: "anthropic" | "openai" | "openai-compatible" | "openai-responses" | "google" | "minimax";
  authType: "none" | "bearer" | "api-key" | "query-param";
  authHeader?: string;  // Custom auth header name
  description?: string;
}

// Provider presets
export const PROVIDER_PRESETS: Record<string, ProviderConfig> = {
  // Official API services
  anthropic: {
    id: "anthropic",
    name: "Anthropic",
    baseUrl: "https://api.anthropic.com",
    apiFormat: "anthropic",
    authType: "api-key",
    description: "Claude Official API",
  },
  openai: {
    id: "openai",
    name: "OpenAI",
    baseUrl: "https://api.openai.com",
    apiFormat: "openai",
    authType: "bearer",
    description: "GPT Official API",
  },
  google: {
    id: "google",
    name: "Google",
    baseUrl: "https://generativelanguage.googleapis.com",
    apiFormat: "google",
    authType: "query-param",
    description: "Gemini Official API",
  },
  minimax: {
    id: "minimax",
    name: "Minimax",
    baseUrl: "https://api.minimax.chat",
    apiFormat: "minimax",
    authType: "bearer",
    description: "Minimax Official API",
  },

  // Local inference services
  ollama: {
    id: "ollama",
    name: "Ollama (Local)",
    baseUrl: "http://localhost:11434",
    apiFormat: "openai-compatible",
    authType: "none",
    description: "Local, free and private",
  },
  localai: {
    id: "localai",
    name: "LocalAI",
    baseUrl: "http://localhost:8080",
    apiFormat: "openai-compatible",
    authType: "none",
    description: "Local, multi-model support",
  },

  // Cloud GPU inference
  vllm: {
    id: "vllm",
    name: "vLLM Server",
    baseUrl: "http://localhost:8000",
    apiFormat: "openai-compatible",
    authType: "none",
    description: "High-performance inference engine",
  },
  tgi: {
    id: "tgi",
    name: "Text Generation Inference",
    baseUrl: "http://localhost:8080",
    apiFormat: "openai-compatible",
    authType: "none",
    description: "HuggingFace inference service",
  },
  sglang: {
    id: "sglang",
    name: "SGLang",
    baseUrl: "http://localhost:30000",
    apiFormat: "openai-compatible",
    authType: "none",
    description: "Structured generation language",
  },

  // API aggregation services
  openrouter: {
    id: "openrouter",
    name: "OpenRouter",
    baseUrl: "https://openrouter.ai/api/v1",
    apiFormat: "openai-compatible",
    authType: "bearer",
    description: "Multi-model aggregation, pay-as-you-go",
  },
  together: {
    id: "together",
    name: "Together AI",
    baseUrl: "https://api.together.xyz/v1",
    apiFormat: "openai-compatible",
    authType: "bearer",
    description: "Open source model cloud service",
  },
  groq: {
    id: "groq",
    name: "Groq",
    baseUrl: "https://api.groq.com/openai/v1",
    apiFormat: "openai-compatible",
    authType: "bearer",
    description: "Ultra-fast inference",
  },
  deepseek: {
    id: "deepseek",
    name: "DeepSeek",
    baseUrl: "https://api.deepseek.com",
    apiFormat: "openai-compatible",
    authType: "bearer",
    description: "DeepSeek Official API",
  },
  siliconflow: {
    id: "siliconflow",
    name: "SiliconFlow",
    baseUrl: "https://api.siliconflow.cn/v1",
    apiFormat: "openai-compatible",
    authType: "bearer",
    description: "Cloud inference service",
  },

  // Custom
  custom: {
    id: "custom",
    name: "Custom Service",
    baseUrl: "http://localhost:8000",
    apiFormat: "openai-compatible",
    authType: "bearer",
    description: "Custom OpenAI-compatible service",
  },
};

export const AVAILABLE_MODELS = [
  // ========== Official API Services ==========
  // Claude Models (Anthropic)
  { id: "claude-opus-4-5-20251101", name: "Claude Opus 4.5", description: "Most capable", provider: "anthropic", baseUrl: "https://api.anthropic.com" },
  { id: "claude-sonnet-4-5-20250929", name: "Claude Sonnet 4.5", description: "Enhanced balanced model", provider: "anthropic", baseUrl: "https://api.anthropic.com" },

  // GPT Models (OpenAI)
  // GPT-5 series uses Responses API
  { id: "gpt-5", name: "GPT-5", description: "Latest flagship model", provider: "openai", baseUrl: "https://api.openai.com", apiFormat: "responses" as const },
  { id: "gpt-5-mini", name: "GPT-5 Mini", description: "Fast and efficient", provider: "openai", baseUrl: "https://api.openai.com", apiFormat: "responses" as const },
  { id: "gpt-5-nano", name: "GPT-5 Nano", description: "Ultra-fast, lightweight", provider: "openai", baseUrl: "https://api.openai.com", apiFormat: "responses" as const },
  // Legacy GPT models use Chat Completions API
  { id: "gpt-4o", name: "GPT-4o", description: "Multimodal model", provider: "openai", baseUrl: "https://api.openai.com" },
  { id: "gpt-4-turbo", name: "GPT-4 Turbo", description: "Fast GPT-4", provider: "openai", baseUrl: "https://api.openai.com" },

  // Gemini Models (Google)
  { id: "gemini-3-pro-preview", name: "Gemini 3 Pro", description: "Google's latest model", provider: "google", baseUrl: "https://generativelanguage.googleapis.com" },

  // Minimax Models
  { id: "minimax-m2.1", name: "Minimax M2.1", description: "Advanced Chinese model", provider: "minimax", baseUrl: "https://api.minimax.chat" },

  // ========== Local Inference (Ollama) ==========
  { id: "llama3.3:latest", name: "Llama 3.3 8B", description: "Meta's latest open source model", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "llama3.3:70b", name: "Llama 3.3 70B", description: "Large model, requires 32GB+ RAM", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "qwen2.5:latest", name: "Qwen 2.5 7B", description: "Alibaba's model, good for Chinese", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "qwen2.5:32b", name: "Qwen 2.5 32B", description: "Large Chinese model", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "deepseek-r1:latest", name: "DeepSeek R1", description: "Strong reasoning capability", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "codellama:latest", name: "Code Llama", description: "Code-specialized model", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "mistral:latest", name: "Mistral 7B", description: "Efficient European model", provider: "ollama", baseUrl: "http://localhost:11434" },
  { id: "phi3:latest", name: "Phi-3", description: "Microsoft small model, efficient", provider: "ollama", baseUrl: "http://localhost:11434" },

  // ========== OpenRouter ==========
  { id: "anthropic/claude-3.5-sonnet", name: "Claude 3.5 Sonnet", description: "via OpenRouter", provider: "openrouter", baseUrl: "https://openrouter.ai/api/v1" },
  { id: "openai/gpt-4o", name: "GPT-4o", description: "via OpenRouter", provider: "openrouter", baseUrl: "https://openrouter.ai/api/v1" },
  { id: "meta-llama/llama-3.3-70b-instruct", name: "Llama 3.3 70B", description: "via OpenRouter", provider: "openrouter", baseUrl: "https://openrouter.ai/api/v1" },
  { id: "deepseek/deepseek-r1", name: "DeepSeek R1", description: "via OpenRouter", provider: "openrouter", baseUrl: "https://openrouter.ai/api/v1" },

  // ========== Together AI ==========
  { id: "meta-llama/Llama-3.3-70B-Instruct-Turbo", name: "Llama 3.3 70B Turbo", description: "via Together", provider: "together", baseUrl: "https://api.together.xyz/v1" },
  { id: "Qwen/Qwen2.5-72B-Instruct-Turbo", name: "Qwen 2.5 72B Turbo", description: "via Together", provider: "together", baseUrl: "https://api.together.xyz/v1" },

  // ========== Groq ==========
  { id: "llama-3.3-70b-versatile", name: "Llama 3.3 70B", description: "via Groq (ultra-fast)", provider: "groq", baseUrl: "https://api.groq.com/openai/v1" },
  { id: "mixtral-8x7b-32768", name: "Mixtral 8x7B", description: "via Groq (ultra-fast)", provider: "groq", baseUrl: "https://api.groq.com/openai/v1" },

  // ========== DeepSeek Official ==========
  { id: "deepseek-chat", name: "DeepSeek Chat", description: "DeepSeek Official", provider: "deepseek", baseUrl: "https://api.deepseek.com" },
  { id: "deepseek-reasoner", name: "DeepSeek Reasoner", description: "Reasoning enhanced", provider: "deepseek", baseUrl: "https://api.deepseek.com" },

  // ========== SiliconFlow ==========
  { id: "Qwen/Qwen2.5-72B-Instruct", name: "Qwen 2.5 72B", description: "via SiliconFlow", provider: "siliconflow", baseUrl: "https://api.siliconflow.cn/v1" },
  { id: "deepseek-ai/DeepSeek-V3", name: "DeepSeek V3", description: "via SiliconFlow", provider: "siliconflow", baseUrl: "https://api.siliconflow.cn/v1" },

  // ========== Custom ==========
  { id: "custom-model", name: "Custom Model", description: "Enter your model ID", provider: "custom", baseUrl: "http://localhost:8000" },
];

const DEFAULT_SETTINGS: Settings = {
  apiKey: "",
  model: "claude-sonnet-4-5-20250929",
  baseUrl: "https://api.anthropic.com",
  maxTokens: 4096,
  temperature: 0.7,
  providerKeys: {},
};

// Get provider ID from model
export function getProviderFromModel(modelId: string): string {
  const model = AVAILABLE_MODELS.find(m => m.id === modelId);
  return model?.provider || "anthropic";
}

// Check if a model uses the OpenAI Responses API (GPT-5 series)
export function usesResponsesApi(modelId: string): boolean {
  // Check if model is in AVAILABLE_MODELS with apiFormat: "responses"
  const model = AVAILABLE_MODELS.find(m => m.id === modelId);
  if (model && 'apiFormat' in model && model.apiFormat === "responses") {
    return true;
  }
  // Fallback: detect GPT-5 models by name pattern
  const lower = modelId.toLowerCase();
  return lower.startsWith("gpt-5") || lower.match(/^gpt-5[\.-]/) !== null;
}

// Convert between frontend and API formats
function fromApiSettings(api: ApiSettings): Settings {
  const providerKeys = api.provider_keys || {};
  const model = api.model;
  const provider = getProviderFromModel(model);

  // Get the current provider's API key
  const apiKey = providerKeys[provider] || api.api_key || "";

  return {
    apiKey,
    model: api.model,
    baseUrl: api.base_url,
    maxTokens: api.max_tokens,
    temperature: api.temperature ?? 0.7,
    providerKeys,
    openaiOrganization: api.openai_organization,
    openaiProject: api.openai_project,
  };
}

function toApiSettings(settings: Settings): ApiSettings {
  // Update the providerKeys with current apiKey for current provider
  const provider = getProviderFromModel(settings.model);
  const providerKeys = { ...settings.providerKeys };
  if (settings.apiKey) {
    providerKeys[provider] = settings.apiKey;
  }

  return {
    api_key: settings.apiKey,
    model: settings.model,
    base_url: settings.baseUrl,
    max_tokens: settings.maxTokens,
    temperature: settings.temperature ?? 0.7,
    provider_keys: providerKeys,
    openai_organization: settings.openaiOrganization,
    openai_project: settings.openaiProject,
  };
}

const [settings, setSettings] = createSignal<Settings>(DEFAULT_SETTINGS);
const [showSettings, setShowSettings] = createSignal(false);
const [isLoading, setIsLoading] = createSignal(true);
const [loadError, setLoadError] = createSignal<string | null>(null);

// Load settings on startup
export async function loadSettings() {
  setIsLoading(true);
  setLoadError(null);
  try {
    const apiSettings = await getSettingsApi();
    setSettings(fromApiSettings(apiSettings));
  } catch (e) {
    const errorMsg = e instanceof Error ? e.message : "Failed to load settings";
    console.error("Failed to load settings:", e);
    setLoadError(errorMsg);
  } finally {
    setIsLoading(false);
  }
}

// Save settings
async function persistSettings(newSettings: Settings) {
  try {
    await saveSettingsApi(toApiSettings(newSettings));
  } catch (e) {
    console.error("Failed to save settings:", e);
  }
}

// Helper function to get model info
export function getModelInfo(modelId: string) {
  return AVAILABLE_MODELS.find(m => m.id === modelId);
}

// Helper function to get default base URL for a model
export function getDefaultBaseUrl(modelId: string): string {
  const model = getModelInfo(modelId);
  return model?.baseUrl || "https://api.anthropic.com";
}

// Check if a provider requires API key
export function providerRequiresApiKey(providerId: string): boolean {
  const config = PROVIDER_PRESETS[providerId];
  if (!config) return true;  // Unknown provider, assume needs key
  return config.authType !== "none";
}

export function useSettings() {
  return {
    settings,
    setSettings,
    showSettings,
    isLoading,
    loadError,
    toggleSettings: () => setShowSettings((v) => !v),
    updateSetting: async <K extends keyof Settings>(key: K, value: Settings[K]) => {
      let newSettings = { ...settings(), [key]: value };

      // When API key changes, also save it to providerKeys for the current provider
      if (key === 'apiKey' && typeof value === 'string') {
        const currentProvider = getProviderFromModel(settings().model);
        newSettings.providerKeys = {
          ...newSettings.providerKeys,
          [currentProvider]: value,
        };
      }

      // When model changes, switch to that provider's stored API key
      if (key === 'model' && typeof value === 'string') {
        const currentModel = getModelInfo(settings().model);
        const newModel = getModelInfo(value);
        const currentProvider = getProviderFromModel(settings().model);
        const newProvider = getProviderFromModel(value);

        // Save current API key to providerKeys before switching
        if (settings().apiKey) {
          newSettings.providerKeys = {
            ...newSettings.providerKeys,
            [currentProvider]: settings().apiKey,
          };
        }

        // Load the new provider's API key
        newSettings.apiKey = newSettings.providerKeys[newProvider] || "";

        // Auto-update base URL if current URL matches the previous model's default
        if (currentModel && newModel && settings().baseUrl === currentModel.baseUrl) {
          newSettings.baseUrl = newModel.baseUrl;
        }
      }

      setSettings(newSettings);
      await persistSettings(newSettings);
    },
    saveAllSettings: async (newSettings: Settings) => {
      // Save current API key to providerKeys
      const provider = getProviderFromModel(newSettings.model);
      if (newSettings.apiKey) {
        newSettings.providerKeys = {
          ...newSettings.providerKeys,
          [provider]: newSettings.apiKey,
        };
      }
      setSettings(newSettings);
      await persistSettings(newSettings);
    },
    // Always show main UI - API key validation happens at request time
    // This allows users to explore the app and switch to local providers without being blocked
    isConfigured: () => true,
    loadSettings,
    getModelInfo,
    getDefaultBaseUrl,
    getProviderFromModel,
    providerRequiresApiKey,
  };
}
