---
layout: home

hero:
  name: Kuse Cowork
  text: AI Agent Framework for Desktop
  tagline: Open-source, cross-platform desktop app with complete privacy control through BYOK integration
  actions:
    - theme: brand
      text: Get Started
      link: /getting-started/installation
    - theme: alt
      text: View on GitHub
      link: https://github.com/kuse-ai/kuse_cowork

features:
  - icon: 🤖
    title: Multi-Provider Support
    details: Connect to Anthropic Claude, OpenAI GPT, Google Gemini, local models (Ollama), and more.
  - icon: 🔄
    title: Autonomous Agent
    details: AI agent that plans and executes tasks step-by-step with real-time progress tracking.
  - icon: 🐳
    title: Secure Isolation
    details: Commands execute in Docker containers, keeping your system safe from unintended changes.
  - icon: 🧩
    title: Extensible Skills
    details: Built-in skills for PDF, DOCX, XLSX processing, with support for custom skill development.
  - icon: 🔌
    title: MCP Protocol
    details: Extend capabilities with Model Context Protocol for external tool integration.
  - icon: 🔒
    title: Privacy First
    details: All data stays local. BYOK (Bring Your Own Key) - you control your API keys.
---

## Why Kuse Cowork?

| Feature | Kuse Cowork | Cloud AI Assistants |
|---------|-------------|---------------------|
| **Privacy** | All data stays local | Data sent to cloud |
| **API Keys** | BYOK - you own your keys | Platform-managed |
| **Offline** | Works with local models | Requires internet |
| **Extensible** | Custom skills & MCP | Limited customization |
| **Open Source** | Fully transparent | Closed source |

## Quick Start

```bash
# Clone and build
git clone https://github.com/kuse-ai/kuse_cowork.git
cd kuse_cowork
pnpm install
pnpm tauri dev
```

[Get Started →](/getting-started/installation)
