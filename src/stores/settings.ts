import { createSignal } from "solid-js";
import {
  getSettings as getSettingsApi,
  saveSettings as saveSettingsApi,
  Settings as ApiSettings,
} from "../lib/tauri-api";

export interface Settings {
  apiKey: string;
  model: string;
  baseUrl: string;
  maxTokens: number;
  temperature?: number;
}

// Provider configuration type
export interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  apiFormat: "anthropic" | "openai" | "openai-compatible" | "google" | "minimax";
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
  { id: "gpt-5.2", name: "GPT 5.2", description: "Latest OpenAI model", provider: "openai", baseUrl: "https://api.openai.com" },
  { id: "gpt-5.1-codex", name: "GPT 5.1 Codex", description: "Code-specialized model", provider: "openai", baseUrl: "https://api.openai.com" },

  // Gemini Models (Google)
  { id: "gemini-3-pro", name: "Gemini 3 Pro", description: "Google's latest model", provider: "google", baseUrl: "https://generativelanguage.googleapis.com" },

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
};

// Convert between frontend and API formats
function fromApiSettings(api: ApiSettings): Settings {
  return {
    apiKey: api.api_key,
    model: api.model,
    baseUrl: api.base_url,
    maxTokens: api.max_tokens,
    temperature: api.temperature ?? 0.7,
  };
}

function toApiSettings(settings: Settings): ApiSettings {
  return {
    api_key: settings.apiKey,
    model: settings.model,
    base_url: settings.baseUrl,
    max_tokens: settings.maxTokens,
    temperature: settings.temperature ?? 0.7,
  };
}

const [settings, setSettings] = createSignal<Settings>(DEFAULT_SETTINGS);
const [showSettings, setShowSettings] = createSignal(false);
const [isLoading, setIsLoading] = createSignal(true);

// Load settings on startup
export async function loadSettings() {
  setIsLoading(true);
  try {
    const apiSettings = await getSettingsApi();
    setSettings(fromApiSettings(apiSettings));
  } catch (e) {
    console.error("Failed to load settings:", e);
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

export function useSettings() {
  return {
    settings,
    setSettings,
    showSettings,
    isLoading,
    toggleSettings: () => setShowSettings((v) => !v),
    updateSetting: async <K extends keyof Settings>(key: K, value: Settings[K]) => {
      let newSettings = { ...settings(), [key]: value };

      // Auto-update base URL when model changes (unless user has custom URL)
      if (key === 'model' && typeof value === 'string') {
        const currentModel = getModelInfo(settings().model);
        const newModel = getModelInfo(value);

        // Only auto-update if current URL matches the previous model's default
        if (currentModel && newModel && settings().baseUrl === currentModel.baseUrl) {
          newSettings.baseUrl = newModel.baseUrl;
        }
      }

      setSettings(newSettings);
      await persistSettings(newSettings);
    },
    saveAllSettings: async (newSettings: Settings) => {
      setSettings(newSettings);
      await persistSettings(newSettings);
    },
    isConfigured: () => settings().apiKey.length > 0,
    loadSettings,
    getModelInfo,
    getDefaultBaseUrl,
  };
}
