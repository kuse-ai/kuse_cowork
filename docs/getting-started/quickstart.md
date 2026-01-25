# Quick Start

This guide will help you get started with Kuse Cowork in minutes.

## Step 1: Configure an AI Provider

After launching Kuse Cowork, you need to configure at least one AI provider.

1. Click the **Settings** icon (⚙️) in the top-right corner
2. Select your preferred AI provider:

::: code-group

```text [Anthropic Claude]
1. Get your API key from https://console.anthropic.com/
2. Enter your API key
3. Select a model (e.g., claude-sonnet-4-5-20250929)
```

```text [OpenAI]
1. Get your API key from https://platform.openai.com/
2. Enter your API key
3. Select a model (e.g., gpt-4o)
```

```text [Local (Ollama)]
1. Install Ollama from https://ollama.ai/
2. Pull a model: ollama pull llama3.3:latest
3. Select "Ollama (Local)" as provider
4. No API key needed!
```

:::

3. Click **Test Connection** to verify
4. Close the settings panel

## Step 2: Start a Chat

The simplest way to use Kuse Cowork is through the Chat interface:

1. Type your message and press Enter
2. The AI will respond with helpful information

**Try asking:**
- "What can you help me with?"
- "Explain how to use Docker"
- "Write a Python script that prints Hello World"

## Example Workflows

### Code Review

```
You: Review the code in src/components/Button.tsx and suggest improvements

Agent: I'll analyze the Button component...
[Tool: read_file src/components/Button.tsx]
[Analysis and suggestions appear]
```

### Bug Investigation

```
You: The tests in tests/api.test.ts are failing. Help me debug.

Agent: Let me investigate the failing tests...
[Tool: read_file tests/api.test.ts]
[Tool: bash npm test -- tests/api.test.ts]
[Debugging steps and fixes]
```

### Code Generation

```
You: Create a new API endpoint for user registration in src/routes/

Agent: I'll create a user registration endpoint...
[Tool: read_file src/routes/index.ts]
[Tool: write_file src/routes/auth.ts]
[New file created with registration logic]
```

## Understanding the Interface

### Chat View

- **Message Input**: Type your messages here
- **Tool Toggle**: Enable/disable tool access
- **Project Selector**: Choose the working directory

### Task View

- **Task List**: All your tasks in the sidebar
- **Progress Panel**: Real-time execution status
- **Plan View**: Step-by-step plan with status indicators
- **Tool Output**: Results from tool executions

### Settings

- **Provider Selection**: Choose AI provider
- **Model Selection**: Pick specific models
- **API Configuration**: Enter API keys and endpoints
- **Docker Status**: Verify container connectivity

## Tips for Better Results

::: tip Be Specific
Instead of "fix the bug", try "fix the TypeError in handleSubmit function in src/Form.tsx"
:::

::: tip Provide Context
Select a project folder so the agent can read your actual code.
:::

::: tip Use Tool Mode for Actions
Enable tools when you want the AI to actually read files or run commands.
:::

::: tip Break Down Large Tasks
For complex projects, create multiple focused tasks rather than one large one.
:::

## Next Steps

- [Configuration Guide](configuration.md) - Detailed settings reference
- [AI Providers](../features/providers.md) - Learn about supported providers
- [Agent System](../features/agent.md) - Deep dive into agent capabilities
- [Tools Reference](../features/tools.md) - Available tools and usage
