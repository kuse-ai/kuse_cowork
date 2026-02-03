

<div align="center">
  <img src="public/kuse-logo.png" alt="Kuse Cowork Logo" width="200"/>
</div>


<br>

<div align="center">

[![DISCORD](https://img.shields.io/badge/Discord-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/Pp5aZjMMAC)

</div>


# Kuse Cowork：An Open-Source, Model-Agnostic [Alternative to Claude Cowork](https://www.claude-cowork.ai/)
>Kuse Cowork is a lightweight, open-source desktop cowork agent built for people who want local-first execution, full model freedom, and real privacy control.

**Works with any models, BYOK, written in Rust** 🚀

[*Demo video: Kuse Cowork in action*](https://github.com/user-attachments/assets/e128e657-c1be-4134-828d-01a9a94ef055)

It is an open-source desktop cowork agent created by [Kuse](https://www.kuse.ai/), an AI document generator & Presentation Maker from your knowledge base. Transform docs, PDFs, YouTube, and images into formatted docs, infographics, mind maps, and flashcards—instantly. Stunning, professional, and ready to use.

## ✨ Why Kuse Cowork?

### 🔐 **BYOK (Bring Your Own Key)**
Use your own API keys or even **bring your own local models** for ultimate privacy control.

### ⚡ **Pure Rust Agent**
Agent fully written in Rust with **zero external dependencies** - blazingly fast and memory-safe.

### 🌍 **Native Cross-Platform**
True native performance on macOS, Windows, and Linux.

### 🛡️ **Container Isolation & Security**
Uses Docker containers for secure command execution and complete isolation.

### 🧩 **Extensible Skills System**
Support for custom skills to extend agent capabilities.
Default skills are: docx, pdf, pptx, xlsx.

### 🔗 **MCP Protocol Support**
Full support for Model Context Protocol (MCP) for seamless tool integration.

---
## 📰 News & Updates

### Standard release
- **\[2026-01-26\]** Release of v0.0.2: Fixing issue where "start task" button actively needs the user to set a cloud-based api key.

### Experimental 
-  **\[2026-01-29\]** Kuse_cowork now supports basic operations on Docx with an integrated UI && support working trace tracking.
-  **\[2026-01-26\]** Kuse_cowork now supports basic operations on Excel with an integrated UI.
 
  
---


## 🚀 Features

- **🔒 Local & Private**: Runs entirely on your machine, API calls go directly to your chosen provider
- **🔑 BYOK Support**: Use your own Anthropic, OpenAI, or local model APIs
- **🎯 Model Agnostic**: Works with Claude, GPT, local models, and more
- **🖥️ Cross-Platform**: macOS (ARM & Intel), Windows, and Linux
- **🪶 Lightweight**: ~10MB app size using Tauri
- **🐳 Containerized**: Docker isolation for enhanced security
- **🧩 Skills**: Extensible skill system for custom capabilities
- **🔗 MCP**: Model Context Protocol support for tool integration

## Security Note
This is still an early project and please be super careful when connecting with your local folders.

## 🚀 Quick Start

Get up and running in minutes:

### 1. Build the project and start

Will update to a clean release build soon. 

### 2. ⚙️ Configure Your AI Model
1. Open **Settings** (gear icon in sidebar)
2. **Choose your AI provider:**
   - **Anthropic Claude** - Enter your Claude API key
   - **OpenAI GPT** - Enter your OpenAI API key
   - **Local Models** - Configure Ollama/LM Studio endpoint
3. **Select your preferred model** (Claude 3.5 Sonnet, GPT-4, etc.)

### 3. 🔑 Enter API Key
- Add your API key in the settings
- Keys are stored locally and never shared

### 4. 📁 Set Workspace Folder
- Click **"Select Project Path"** when creating a new task
- Choose your project folder or workspace directory
- The agent will work within this folder context

### 5. 🎯 Start Your First Task!
1. Click **"New Task"**
2. Describe what you want to accomplish
3. Watch the AI agent work on your project
4. Review the plan and implementation steps

**Example tasks:**
- *"Organize my folders"*
- *"Read all the receipts and make an expense reports"*
- *"Summarize the meeting notes and give me all the TODOs."*


---

## 🛠️ Development

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (for Tauri)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (required for container isolation)
- [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

**Note**: Docker Desktop must be installed and running for container isolation features. Without Docker, the app will still work but commands will run without isolation.

### Setup

```bash
# Clone the repo
git clone https://github.com/kuse-ai/kuse_cowork.git
cd kuse_cowork

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Project Structure

```
kuse_cowork/
├── src/                    # Frontend (SolidJS + TypeScript)
│   ├── components/         # UI components
│   ├── lib/               # Utilities (API clients, MCP)
│   └── stores/            # State management
├── src-tauri/             # Backend (Rust + Tauri)
│   ├── src/               # Rust source code
│   │   ├── agent/         # Agent implementation
│   │   ├── tools/         # Built-in tools
│   │   ├── skills/        # Skills system
│   │   ├── mcp/           # MCP protocol support
│   │   └── database.rs    # Local data storage
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── .github/workflows/     # CI/CD for cross-platform builds
└── docs/                  # Documentation and assets
```

## 🔧 Configuration

### API Providers

Kuse Cowork supports multiple AI providers:

- **Anthropic Claude**: Direct API integration
- **OpenAI GPT**: Full GPT model support
- **Local Models**: Ollama, LM Studio, or any OpenAI-compatible endpoint
- **Custom APIs**: Configure any compatible endpoint

### Settings

All settings are stored locally and never shared:

- **API Configuration**: Keys and endpoints for your chosen provider
- **Model Selection**: Choose from available models
- **Agent Behavior**: Temperature, max tokens, system prompts
- **Security**: Container isolation settings
- **Skills**: Enable/disable custom skills
- **MCP Servers**: Configure external tool providers

## 🛡️ Security & Privacy

### Container Isolation
Kuse Cowork uses Docker containers to isolate all external command execution:
- **Complete isolation** from your host system
- **Secure networking** with controlled access
- **Resource limits** to prevent abuse
- **Clean environments** for each execution

### Privacy First
- **No telemetry** - nothing is sent to our servers
- **Local storage** - all data stays on your machine
- **Direct API calls** - communications only with your chosen AI provider
- **Open source** - full transparency of all code
## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🏗️ Built With

- **[Tauri](https://tauri.app/)** - Lightweight desktop framework
- **[Rust](https://rust-lang.org/)** - Systems programming language

## 🚧 Roadmap & TODOs

### Upcoming Features
- **📦 Streamlined Release Pipeline** - Automated builds and easier distribution
- **🎯 Simplified Setup** - One-click installation for non-developers
- **🐬 Lightweight Sandbox** - Migrate to an lightweight sandbox.
- **🧠 Context Engineering** - Enhanced support for better context management
- **🔧 Auto-configuration** - Intelligent setup for common development environments
- **📱 Mobile Support** - Cross-platform mobile app support

### Current Limitations
- Docker Desktop required for full isolation features
- Manual setup process for development environment

## 🚄 Use Cases

### File and Document Management

1. Receipt Processing → Expense Reports
  - Drop receipts into a folder and ask Kuse to generate a formatted expense report. -> See [Results](https://hosting.kuse.ai/pages/HxcKeoc9qzgz53GukJWT5a.html?_gl=1*1fkzfdd*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
2. Contract & Document Consolidation
  - Merge multiple drafts, addendums, and feedback notes into one clean final version. -> See [Results](https://www.kuse.ai/pages/EcoGrow-Final-Launch-Execution-Plan-Official-Document/jKXXcarFu4N4AYN8FeTmVr?_gl=1*1od9b1t*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
3. Instant Metadata Tagging
  - Automatically extract metadata from large batches of documents. -> See [Results](https://www.kuse.ai/pages/Document-Metadata-Extraction-Dashboard/GUuVJMvHckCRecTJNGKpnV?_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)

### Research & Analysis
1. Research Synthesis
  - Combine articles, papers, notes, and web sources into coherent reports. -> See [Results](https://hosting.kuse.ai/pages/f88yBecR8fn8Lw4KP9zgXV.html?board_id=cx8cDS7TJ4gQGYbRYz8SqC&creator_anon_id=98040&_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
2. Transcript Analysis
  - Extract themes, action items, and key insights from transcripts. -> See [Results](https://hosting.kuse.ai/pages/bddVPFxzAeNDwL6dTqDxPt.html?board_id=GAf3KaQEHK9PNki5ekh7Te&creator_anon_id=98040&_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
3. Personal Knowledge Synthesis
  - Analyze journals, notes, and research files to uncover hidden patterns. -> See [Results](https://www.kuse.ai/pages/Cross-File-Pattern-Analysis:-Cognitive-Work,-Information-Management-&-Digital-Wellbeing/dahfAT9KiKQ9zAWewnTcmu?_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)

### Document & Presentation Creation
1. Spreadsheets with Formulas
  - Generate real Excel files — not broken CSVs. -> See [Results](https://www.kuse.ai/pages/SaaS-Executive-Dashboard/BcFGhbtWFzmCx54EVwCeCn?_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
2. Presentations from Notes
  - Create slide decks from transcripts or rough outlines. -> See [Results](https://hosting.kuse.ai/pages/hQ8n9qjD6eHASAa5HsbNpV.html?board_id=TGnuJaWVPWQ8yVtUz49eBx&creator_anon_id=98040&_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
3. Reports from Messy Inputs
  - Turn voice memos and scattered notes into polished documents. -> See [Results](https://www.kuse.ai/pages/Cross-File-Pattern-Analysis:-Cognitive-Work,-Information-Management-&-Digital-Wellbeing/dahfAT9KiKQ9zAWewnTcmu?_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)

### Data & Analysis
1. Statistical Analysis
  - Outlier detection, cross-tabulation, time-series insights. -> See [Results](https://www.kuse.ai/pages/SaaS-Executive-Dashboard/BcFGhbtWFzmCx54EVwCeCn?_gl=1*pf14ll*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
2. Data Visualization
  - Generate presentation-ready charts and dashboards. -> See [Results](https://www.kuse.ai/pages/NVIDIA-Financial-Performance-Dashboard/N7977gdYrESSLeKNGYKi2C?_gl=1*afop29*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)
3. Data Transformation
  - Clean and restructure messy datasets into standardized formats. -> See [Results](https://www.kuse.ai/pages/Marketing-Campaign-Dashboard-Kuse-AI/47mS8HoUL6cQidSQWb2FXh?_gl=1*afop29*_ga*OTc1MzA3ODIuMTc2ODc5NjY3MA..*_ga_3806BK9P0R*czE3Njg5ODY3OTAkbzEyJGcxJHQxNzY4OTg4NjczJGo2MCRsMCRoODMzNTI5Mzk0)


## 🙏 Credits

Inspired by:
- **[Claude Cowork](https://claude.com/blog/cowork-research-preview)** - The original inspiration

---
**⭐ Star this repo if you find it useful!**
---
**For bugs, feature requests, or roadmap discussions, please open a GitHub Issue.**

---
## Documentation & Guides
- Open cowork Hub: [https://www.open-cowork.io](https://www.open-cowork.io)



