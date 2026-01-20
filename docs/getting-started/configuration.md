# Configuration

This guide covers all configuration options in Kuse Cowork.

## Settings Overview

Access settings by clicking 'Settings' in the bottom-left corner of the application.

## AI Provider Configuration

### Provider Selection

Kuse Cowork supports multiple AI providers:

| Provider | Type | API Key Required |
|----------|------|------------------|
| Anthropic | Official API | Yes |
| OpenAI | Official API | Yes |
| Google | Official API | Yes |
| Minimax | Official API | Yes |
| DeepSeek | Official API | Yes |
| Ollama | Local | No |
| LocalAI | Local | No |
| OpenRouter | Aggregator | Yes |
| Together AI | Aggregator | Yes |
| Groq | Aggregator | Yes |
| SiliconFlow | Aggregator | Yes |

### API Key Management

API keys are stored locally in an encrypted SQLite database. Keys are never sent to any server other than the respective AI provider.

```
~/.kuse-cowork/settings.db
```

::: info API Key Safety
- Keys are stored locally only
- No telemetry or analytics
- Keys are passed directly to provider APIs
:::

### Model Selection

Each provider offers multiple models:

**Anthropic:**

| Model | Description |
|-------|-------------|
| claude-opus-4-5-20251101 | Most capable |
| claude-sonnet-4-5-20250929 | Balanced (recommended) |

**OpenAI:**

| Model | Description |
|-------|-------------|
| gpt-5 | Latest flagship |
| gpt-5-mini | Fast and efficient |
| gpt-4o | Multimodal |

**Local (Ollama):**

| Model | Description |
|-------|-------------|
| llama3.3:latest | Meta's latest open source |
| qwen2.5:latest | Good for Chinese |
| codellama:latest | Code-specialized |
| deepseek-r1:latest | Strong reasoning |

### Custom Endpoints

For self-hosted or enterprise deployments:

1. Select "Custom Service" as provider
2. Enter your base URL (e.g., `http://localhost:8000`)
3. Configure authentication if needed
4. Enter your model ID

## Application Settings

### Max Tokens

Controls the maximum length of AI responses:

- **Default**: 4096 tokens
- **Range**: 256 - 128000 tokens
- **Recommendation**: 4096 for chat, 8192+ for code generation

### Temperature

Controls response randomness:

- **0.0**: Deterministic, focused
- **0.7**: Balanced (default)
- **1.0**: Creative, varied

::: info Reasoning Models
GPT-5, o1, and o3 models don't support custom temperature.
:::

## Project Configuration

### Project Folder

Set the working directory for tasks:

1. Click "Select Folder" in task creation
2. Browse to your project root
3. The agent will have access to all files in this directory

::: warning Docker Mounting
The project folder is mounted at `/workspace` in Docker containers.
:::

### Skills Directory

Custom skills are stored in:

```
~/.kuse-cowork/skills/
```

See [Skills Guide](../features/skills.md) for creating custom skills.

## Docker Configuration

### Container Settings

Kuse Cowork uses Docker for safe command execution:

| Setting | Value |
|---------|-------|
| Default Image | `python:3.11-alpine` |
| Available Images | `ubuntu:latest`, `node:20`, `rust:alpine` |
| Workspace Mount | `/workspace` |
| Skills Mount | `/skills` (read-only) |

### Verifying Docker

Check Docker status in Settings. If disconnected:

1. Ensure Docker Desktop is running
2. Check Docker socket permissions
3. Restart Kuse Cowork

## MCP Configuration

### Adding MCP Servers

1. Go to Settings → MCP
2. Click "Add Server"
3. Enter server configuration:

```json
{
  "name": "my-mcp-server",
  "url": "http://localhost:3000",
  "auth": {
    "type": "bearer",
    "token": "your-token"
  }
}
```

### OAuth MCP Servers

For OAuth-authenticated MCP servers:

1. Click "Connect with OAuth"
2. Complete authentication in browser
3. Token is stored locally

## Data Storage

All data is stored locally:

| Data | Location |
|------|----------|
| Settings | `~/.kuse-cowork/settings.db` |
| Conversations | `~/.kuse-cowork/settings.db` |
| Tasks | `~/.kuse-cowork/settings.db` |
| Skills | `~/.kuse-cowork/skills/` |

### Backup

To backup your data:

```bash
cp -r ~/.kuse-cowork ~/.kuse-cowork-backup
```

### Reset

To reset all settings:

```bash
rm -rf ~/.kuse-cowork
```

## Environment Variables

For advanced configuration, environment variables can be used:

| Variable | Description |
|----------|-------------|
| `KUSE_DATA_DIR` | Override data directory |
| `KUSE_LOG_LEVEL` | Set log level (debug, info, warn, error) |

## Configuration File

Settings can also be configured via JSON:

```json
// ~/.kuse-cowork/config.json
{
  "defaultProvider": "anthropic",
  "defaultModel": "claude-sonnet-4-5-20250929",
  "maxTokens": 4096,
  "temperature": 0.7,
  "dockerEnabled": true
}
```

## Next Steps

- [AI Providers](../features/providers.md) - Detailed provider documentation
- [Skills](../features/skills.md) - Create custom skills
- [MCP Protocol](../features/mcp.md) - Connect external tools
