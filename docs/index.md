# Kuse Cowork

**Open-source AI Agent Framework for Desktop**

A cross-platform desktop application that brings AI agent capabilities to your local machine with complete privacy control.

---

## What is Kuse Cowork?

Kuse Cowork is an open-source, cross-platform desktop application that functions as an AI agent framework. It's designed as an alternative to cloud-based AI assistants, allowing users to leverage AI agents for software development and productivity tasks with **complete privacy control** through BYOK (Bring Your Own Key) integration.

The agent can:

- **Read and write files** in your project
- **Execute commands** in isolated Docker containers
- **Search and navigate** codebases
- **Access extended tools** through MCP protocol
- **Plan and execute** multi-step tasks autonomously

## Key Features

### Multi-Provider Support
Connect to Anthropic Claude, OpenAI GPT, Google Gemini, local models (Ollama), and more.
[Learn more](features/providers.md)

### Autonomous Agent
AI agent that plans and executes tasks step-by-step with real-time progress tracking.
[Learn more](features/agent.md)

### Secure Isolation
Commands execute in Docker containers, keeping your system safe from unintended changes.
[Learn more](features/tools.md)

### Extensible Skills
Built-in skills for PDF, DOCX, XLSX processing, with support for custom skill development.
[Learn more](features/skills.md)

## Quick Start

### macOS / Linux / Windows

```bash
# Download from GitHub Releases
# Or build from source:
git clone https://github.com/kuse-ai/kuse_cowork.git
cd kuse_cowork
pnpm install
pnpm tauri build
```

## Why Kuse Cowork?

| Feature | Kuse Cowork | Cloud AI Assistants |
|---------|-------------|---------------------|
| **Privacy** | All data stays local | Data sent to cloud |
| **API Keys** | BYOK - you own your keys | Platform-managed |
| **Offline** | Works with local models | Requires internet |
| **Extensible** | Custom skills & MCP | Limited customization |
| **Open Source** | Fully transparent | Closed source |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (SolidJS)                       │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │  User Interface │  │  State Stores   │                   │
│  └────────┬────────┘  └────────┬────────┘                   │
└───────────┼────────────────────┼────────────────────────────┘
            │                    │
            ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                   Backend (Rust/Tauri)                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │  Agent   │ │   LLM    │ │  Tools   │ │   MCP    │        │
│  │   Loop   │ │  Client  │ │ Executor │ │ Manager  │        │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘        │
└───────┼────────────┼────────────┼────────────┼──────────────┘
        │            │            │            │
        ▼            ▼            ▼            ▼
┌──────────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│ AI Providers │ │  SQLite  │ │  Docker  │ │   MCP    │
│(Claude, GPT) │ │    DB    │ │Containers│ │ Servers  │
└──────────────┘ └──────────┘ └──────────┘ └──────────┘
```

## Community

- [GitHub Repository](https://github.com/kuse-ai/kuse_cowork)
- [Issue Tracker](https://github.com/kuse-ai/kuse_cowork/issues)
- [Contributing Guide](development/contributing.md)

## License

Kuse Cowork is released under the MIT License. See [LICENSE](https://github.com/kuse-ai/kuse_cowork/blob/main/LICENSE) for details.
