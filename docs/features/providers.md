# AI Providers

Kuse Cowork supports multiple AI providers, giving you flexibility in choosing the right model for your needs.

## Supported Providers

### Official API Providers

These providers require API keys from the respective companies.

#### Anthropic Claude

The default and recommended provider for most tasks.

| Model | Description | Best For |
|-------|-------------|----------|
| `claude-opus-4-5-20251101` | Most capable | Complex reasoning, creative tasks |
| `claude-sonnet-4-5-20250929` | Balanced | General use (recommended) |

**Configuration:**

```
Base URL: https://api.anthropic.com
Auth: x-api-key header
```

Get your API key at [console.anthropic.com](https://console.anthropic.com/)

#### OpenAI

Support for GPT models including the latest GPT-5 series.

| Model | Description | API Format |
|-------|-------------|------------|
| `gpt-5` | Latest flagship | Responses API |
| `gpt-5-mini` | Fast and efficient | Responses API |
| `gpt-5-nano` | Ultra-fast | Responses API |
| `gpt-4o` | Multimodal | Chat Completions |
| `gpt-4-turbo` | Fast GPT-4 | Chat Completions |

!!! note "GPT-5 Responses API"
    GPT-5 models use OpenAI's new Responses API format, which is automatically detected and handled.

Get your API key at [platform.openai.com](https://platform.openai.com/)

#### Google Gemini

Google's latest AI models with thinking capabilities.

| Model | Description |
|-------|-------------|
| `gemini-3-pro-preview` | Google's latest model |

**Special Features:**

- Thinking/reasoning mode with `thoughtSignature` support
- Function calling with thought signatures

Get your API key at [ai.google.dev](https://ai.google.dev/)

#### Minimax

Advanced Chinese language model provider.

| Model | Description |
|-------|-------------|
| `minimax-m2.1` | Advanced Chinese model |

### Local Inference

Run models locally for privacy and offline use.

#### Ollama

The easiest way to run local models.

**Setup:**

```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Pull a model
ollama pull llama3.3:latest
```

**Available Models:**

| Model | Size | Description |
|-------|------|-------------|
| `llama3.3:latest` | 8B | Meta's latest |
| `llama3.3:70b` | 70B | Requires 32GB+ RAM |
| `qwen2.5:latest` | 7B | Good for Chinese |
| `deepseek-r1:latest` | Various | Strong reasoning |
| `codellama:latest` | 7B | Code-specialized |
| `mistral:latest` | 7B | Efficient European model |
| `phi3:latest` | 3.8B | Microsoft small model |

**Configuration:**

```
Base URL: http://localhost:11434
Auth: None required
```

#### LocalAI

OpenAI-compatible local inference server.

```
Base URL: http://localhost:8080
Auth: None required
```

#### vLLM / SGLang / TGI

High-performance inference servers:

| Server | Default Port | Description |
|--------|--------------|-------------|
| vLLM | 8000 | High-performance inference |
| SGLang | 30000 | Structured generation |
| TGI | 8080 | HuggingFace inference |

### Aggregation Services

Access multiple models through a single API.

#### OpenRouter

Access 100+ models through one API.

| Model | Description |
|-------|-------------|
| `anthropic/claude-3.5-sonnet` | Claude via OpenRouter |
| `openai/gpt-4o` | GPT-4o via OpenRouter |
| `meta-llama/llama-3.3-70b-instruct` | Llama 3.3 |
| `deepseek/deepseek-r1` | DeepSeek R1 |

Get your API key at [openrouter.ai](https://openrouter.ai/)

#### Groq

Ultra-fast inference with specialized hardware.

| Model | Description |
|-------|-------------|
| `llama-3.3-70b-versatile` | Llama 3.3 70B |
| `mixtral-8x7b-32768` | Mixtral MoE |

Get your API key at [console.groq.com](https://console.groq.com/)

#### Together AI

Cloud inference for open-source models.

| Model | Description |
|-------|-------------|
| `meta-llama/Llama-3.3-70B-Instruct-Turbo` | Llama 3.3 Turbo |
| `Qwen/Qwen2.5-72B-Instruct-Turbo` | Qwen 2.5 Turbo |

#### DeepSeek

Chinese AI provider with strong coding models.

| Model | Description |
|-------|-------------|
| `deepseek-chat` | General chat |
| `deepseek-reasoner` | Enhanced reasoning |

#### SiliconFlow

Cloud inference service with Chinese model focus.

| Model | Description |
|-------|-------------|
| `Qwen/Qwen2.5-72B-Instruct` | Qwen 2.5 |
| `deepseek-ai/DeepSeek-V3` | DeepSeek V3 |

## Provider Configuration

### Switching Providers

1. Open Settings (⚙️)
2. Select provider from the dropdown
3. Enter API key (if required)
4. Select model
5. Click "Test Connection"

### API Key Storage

API keys are stored in:

```
~/.kuse-cowork/settings.db
```

Keys are:

- Stored locally only
- Never sent to third parties
- Associated with specific providers

### Per-Provider Keys

You can configure different API keys for each provider:

```json
{
  "providerKeys": {
    "anthropic": "sk-ant-...",
    "openai": "sk-...",
    "openrouter": "sk-or-..."
  }
}
```

When switching models, the appropriate key is automatically selected.

## Custom Providers

### OpenAI-Compatible Endpoints

Connect to any OpenAI-compatible API:

1. Select "Custom Service" as provider
2. Enter base URL
3. Configure authentication
4. Enter model ID

**Example: LM Studio**

```
Base URL: http://localhost:1234/v1
Auth: None
Model: local-model
```

### Enterprise Deployments

For Azure OpenAI or self-hosted deployments:

```
Base URL: https://your-deployment.openai.azure.com
Auth: Bearer token
Model: your-deployment-name
```

## Reasoning Models

Some models have special requirements:

### Temperature Restrictions

The following models don't support custom temperature:

- OpenAI: `o1-*`, `o3-*`, `gpt-5*`
- DeepSeek: `deepseek-reasoner`

Temperature is automatically ignored for these models.

### Extended Thinking

Some models support extended thinking/reasoning:

- Gemini 3: Uses `thoughtSignature` for function calling
- Claude: Uses extended thinking mode

## Best Practices

### Choosing a Provider

| Use Case | Recommended Provider |
|----------|---------------------|
| General coding | Claude Sonnet |
| Complex reasoning | Claude Opus or GPT-5 |
| Fast iteration | Groq or Ollama |
| Privacy-focused | Local models (Ollama) |
| Cost optimization | OpenRouter |
| Chinese content | Qwen or DeepSeek |

### Cost Management

- Use smaller models for simple tasks
- Use local models for development/testing
- Monitor usage through provider dashboards

### Performance Tips

- Groq offers fastest cloud inference
- Ollama is fastest for local (if you have GPU)
- Use streaming for better UX

## Troubleshooting

??? question "Connection test fails"

    1. Verify API key is correct
    2. Check base URL format
    3. Ensure network connectivity
    4. Check provider status page

??? question "Model not found"

    1. Verify model ID spelling
    2. Check if model is available in your plan
    3. For Ollama, ensure model is pulled

??? question "Rate limit errors"

    1. Reduce request frequency
    2. Upgrade provider plan
    3. Use multiple provider keys

## Next Steps

- [Agent System](agent.md) - Learn how the agent uses models
- [Tools](tools.md) - Understand tool execution
- [Configuration](../getting-started/configuration.md) - Detailed settings
