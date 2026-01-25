# Features Overview

Kuse Cowork is a comprehensive AI agent framework designed for developers who need powerful automation with complete privacy control.

## Core Features

###  Multi-Provider AI Support

Connect to any major AI provider or run models locally:

- **Official APIs**: Anthropic Claude, OpenAI GPT, Google Gemini, Minimax
- **Local Inference**: Ollama, LocalAI, vLLM, Text Generation Inference
- **Aggregators**: OpenRouter, Together AI, Groq, SiliconFlow, DeepSeek
- **Custom Endpoints**: Any OpenAI-compatible API

[Learn more →](providers.md)

###  Autonomous Agent System

An intelligent agent that can plan and execute complex tasks:

- **Automatic Planning**: Analyzes tasks and creates step-by-step plans
- **Tool Orchestration**: Uses the right tools for each step
- **Progress Tracking**: Real-time visibility into execution status
- **Context Awareness**: Maintains context across conversation turns

[Learn more →](agent.md)

###  Comprehensive Tool Suite

Built-in tools for common development tasks:

| Tool | Description |
|------|-------------|
| `read_file` | Read file contents |
| `write_file` | Create or overwrite files |
| `edit_file` | Make targeted edits |
| `bash` | Execute shell commands |
| `glob` | Find files by pattern |
| `grep` | Search file contents |
| `list_dir` | List directory contents |
| `docker_run` | Run containers |

[Learn more →](tools.md)

###  Extensible Skills System

Extend agent capabilities with skills:

- **Built-in Skills**: PDF, DOCX, XLSX, PPTX processing
- **Custom Skills**: Create your own skills with simple YAML/Markdown
- **Auto-mounting**: Skills available in Docker containers at `/skills`
- **Script Execution**: Python, Node.js, and other runtime support

[Learn more →](skills.md)

###  MCP Protocol Support

Connect external tools and services:

- **Dynamic Tools**: Discover and use tools from MCP servers
- **OAuth Support**: Secure authentication with external services
- **Real-time Status**: Monitor server connections
- **Error Handling**: Graceful fallbacks and retries

[Learn more →](mcp.md)

## Security Features

### Local-First Architecture

All data stays on your machine:

-  SQLite for local storage
-  API keys stored locally only
-  No telemetry or analytics
-  Works offline with local models

### Docker Isolation

Commands execute in isolated containers:

-  Sandboxed execution environment
-  Controlled filesystem access
-  Resource limits and timeouts
-  Clean container per execution

### BYOK (Bring Your Own Key)

Complete control over your AI access:

-  Use your own API keys
-  Connect to your own endpoints
-  Pay only for what you use
-  Switch providers anytime

## User Interface Features

### Chat Interface

Simple conversational interaction:

- **Markdown Rendering**: Rich text formatting
- **Code Highlighting**: Syntax highlighting for 100+ languages
- **Tool Toggle**: Enable/disable tool access
- **Message History**: Persistent conversation history

### Task Management

Organize complex work:

- **Task Creation**: Describe what you want to accomplish
- **Project Context**: Associate tasks with project folders
- **Progress Panel**: Real-time execution status
- **History**: Review past tasks and results

### Settings Panel

Easy configuration:

- **Provider Selection**: Visual model picker
- **Connection Testing**: Verify API connectivity
- **MCP Management**: Add and manage MCP servers
- **Docker Status**: Monitor container connectivity

## Integration Features

### Project Integration

Work with your existing projects:

- **Folder Selection**: Pick any directory as workspace
- **Codebase Awareness**: Agent understands project structure
- **File Operations**: Read, write, and edit project files
- **Build Integration**: Run build commands and tests

### IDE-like Features

Developer-focused functionality:

- **Code Search**: Find code with glob and grep
- **File Navigation**: Browse project structure
- **Diff View**: See changes before applying
- **Error Detection**: Parse and explain errors

## Performance Features

### Streaming Responses

Real-time output:

- **Token Streaming**: See responses as they generate
- **Progress Events**: Track tool execution
- **Incremental Updates**: UI updates in real-time

### Efficient Execution

Optimized for developer workflows:

- **Parallel Tool Calls**: Execute multiple tools simultaneously
- **Context Caching**: Reuse relevant context
- **Smart Truncation**: Handle large files gracefully

## Platform Support

### Cross-Platform

Runs on all major operating systems:

-  macOS (Intel and Apple Silicon)
-  Windows 10/11
-  Linux (Ubuntu, Fedora, Arch)

### Native Performance

Built with Rust and Tauri:

- **Small Binary**: ~20MB download size
- **Low Memory**: Efficient resource usage
- **Fast Startup**: Opens in seconds
- **Native Look**: Follows system theme

## What's Next?

Explore each feature in detail:

- [AI Providers](providers.md) - Configure and use different AI services
- [Agent System](agent.md) - Understand autonomous task execution
- [Tools](tools.md) - Learn about available tools
- [Skills](skills.md) - Extend with custom capabilities
- [MCP Protocol](mcp.md) - Connect external services
