# iAgent Windows

<div align="center">

### Autonomous AI Agent Runtime for Windows

Persistent desktop AI orchestration with local execution, ambient workflows, provider routing, memory systems, and tool-driven automation.
iAgent is a next generation autonomous AI agent platform fully integrated into Windows as an ambient agent providing suggestions and minimally intrusive chat dock to help you accomplish more in your tasks, think co-working and full agentic building/researching activities. It can also interact easily with office Tools (Word, Excel, Powerpoint), web Tools (search for you, fill forms,...). It learns you preferences, evolves thanks to its deep memory layer.
It's always available, has computer use and full agentic capabilities (with swarm agents) but remains in the background for you to focus on what you need to achieve!

![iAgent Infographic](img/1000006999.png)

</div>

---

## Screenshots

![iAgent Desktop](docs/assets/iagent-desktop-full.jpg)

---

# Overview

iAgent Windows is a local-first ambient AI runtime designed for persistent desktop workflows.

Unlike browser-only assistants or stateless chatbot wrappers, iAgent behaves like a continuously available execution environment capable of:

- interacting with the local machine
- orchestrating desktop workflows
- executing shell commands
- operating on files and projects
- maintaining persistent sessions and memory
- running background and ambient jobs
- coordinating provider-backed reasoning
- integrating directly into Windows UX

The platform combines:

- a modular Rust async runtime
- desktop dock and overlay interfaces
- provider abstraction layers
- persistent memory and storage systems
- execution planning pipelines
- tooling orchestration
- local-first execution
- ambient automation

---

# Prerequisites

Before installing or building iAgent, ensure you have:

## Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | 1.70+ | Compile the Rust runtime |
| **Git** | any recent | Clone and manage repository |
| **PowerShell** | 5.1+ | Windows launcher scripts |

## Required API Keys

iAgent requires at least one LLM provider API key:

| Provider | Environment Variable | Signup |
|----------|---------------------|--------|
| OpenAI | `OPENAI_API_KEY` | https://platform.openai.com |
| OpenRouter | `OPENROUTER_API_KEY` | https://openrouter.ai |
| Gemini | `GEMINI_API_KEY` | https://aistudio.google.com |

For GitHub operations (optional): Generate a PAT at https://github.com/settings/tokens

---

# Configuration

## Environment Variables

Create a `.env` file or set environment variables:

```bash
# Required: At least one provider API key
OPENAI_API_KEY=sk-...
# OR
OPENROUTER_API_KEY=sk-or-...

# Optional: Provider selection (default: openai)
IAGENT_PROVIDER=openai

# Optional: Model selection (default: gpt-4o)
IAGENT_MODEL=gpt-4o

# Optional: Residential proxy for GitHub operations (prevents bot detection)
HTTPS_PROXY=http://user:pass@proxyhost:port
HTTP_PROXY=http://user:pass@proxyhost:port

# Optional: Log level (default: info)
RUST_LOG=info
```

## Provider Selection

```bash
# Use OpenAI
IAGENT_PROVIDER=openai

# Use OpenRouter
IAGENT_PROVIDER=openrouter

# Use Gemini
IAGENT_PROVIDER=gemini
```

## Proxy Setup (Stealth Mode)

If using automated GitHub operations, configure a residential proxy to avoid bot detection:

```bash
# Set proxy environment variables
export HTTPS_PROXY=http://user:pass@proxyhost:port
export HTTP_PROXY=http://user:pass@proxyhost:port
```

**Recommended proxy providers**: Luminati, SmartProxy, Oxylabs (residential rotating proxies)

---

# Core Capabilities

## Autonomous Execution

The runtime is designed around execution-first agent behavior.

Agents can:

- plan actions
- dispatch tools
- operate on projects
- execute commands
- iterate on tasks
- maintain contextual continuity

---

## Persistent Memory

Dedicated memory and storage layers enable:

- persistent sessions
- contextual continuity
- structured knowledge
- long-running workflows
- memory-aware orchestration

---

## Tool Ecosystem

Integrated tooling includes:

- filesystem access
- shell execution
- web context tooling
- planning systems
- integration layers
- memory tooling
- desktop automation

---

## Desktop Integrations

iAgent connects directly to the Windows desktop and key productivity applications through three integration layers.

### Windows Desktop Automation

The runtime can control Windows applications via Chrome DevTools Protocol (CDP), communicating directly with running Chrome or Edge browsers. This enables:

- **Tab management** — list open tabs, open new tabs, navigate to URLs
- **DOM inspection** — find clickable elements, forms, buttons, and text fields
- **Browser actions** — click elements, type text, evaluate JavaScript, capture screenshots
- **Form automation** — fill and submit web forms automatically from structured field data

Launch Chrome or Edge with `--remote-debugging-port=9222` to enable the agent's browser control. All browser actions work against live browser sessions — no screenshot-based OCR or X11 forwarding needed.

### Web & Form Automation

The form-fill engine extracts interactive elements from any webpage and can populate them from structured input. Use it for:

- Autofill data entry on web-based administrative tools
- Batch-fill repetitive forms from CSV or structured input
- Automated data submission to internal web portals

### Office Documents (Word, Excel, PowerPoint)

iAgent manipulates Office documents directly via [OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) — a self-contained cross-platform binary that reads and writes `.docx`, `.xlsx`, and `.pptx` files without requiring Microsoft Office to be installed.

**Word (.docx)**

- Create new documents, open existing files
- Read and extract plain text from any position in the document
- Get document statistics: paragraph count, word count, page count
- Insert paragraphs and text with optional style formatting
- Find and replace text throughout a document (plain or regex)
- Format matched text (bold, color, style)
- Remove elements by path
- Validate against OpenXML schema
- Export to HTML

**Excel (.xlsx)**

- Get and set cell values by address (e.g. `Sheet1!A1`)
- Insert formulas into cells
- Read cell ranges as JSON
- Get sheet statistics: rows, columns, sheets
- Batch update multiple cells from structured data
- Open in resident mode to prevent file lock conflicts

**PowerPoint (.pptx)**

- Add slides with configurable layouts
- Add textboxes to any slide with position and content
- Set shape properties (fill, outline, font)
- Get all shapes on a slide with their properties
- Read slide text and content

**Batch operations** — run multi-step document workflows from a single JSON command batch (e.g. open 50 Excel files, update a header row, save and close).

All Office operations return structured JSON output and work on Windows, macOS, and Linux.

---

## Multi-Provider Runtime

Provider abstraction enables routing across:

- OpenAI
- OpenRouter
- Gemini
- AWS Bedrock-related infrastructure

---

# Architecture

```mermaid
flowchart TB

    subgraph User["User Layer"]
        USER[Desktop User]
        VOICE[Voice / Typed Input]
        HOTKEY[Alt+; Launcher]
    end

    subgraph Windows["Windows Integration"]
        SHORTCUT[iAgent.lnk]
        VBS[Hidden VBS Launcher]
        PS[PowerShell Runtime Launcher]
    end

    subgraph Frontend["Desktop Dock Frontend"]
        DOCK[Dock / Tray UI]
        OVERLAY[Overlay UI]
        INBOX[Task Inbox]
    end

    subgraph Runtime["Rust Runtime Engine"]
        SERVER[Runtime Server]
        AGENT[Agent Executor]
        MEMORY[Memory System]
        SESSION[Session Manager]
        PLAN[Planning Engine]
        AMBIENT[Ambient Jobs]
    end

    subgraph Tooling["Tool Execution Layer"]
        FS[Filesystem Tools]
        SHELL[Shell Execution]
        WEB[Web Context]
        INT[Integrations]
    end

    subgraph Providers["Provider Layer"]
        OPENAI[OpenAI]
        OPENROUTER[OpenRouter]
        GEMINI[Gemini]
    end

    USER --> HOTKEY
    USER --> VOICE
    HOTKEY --> SHORTCUT --> VBS --> PS
    PS --> DOCK
    DOCK --> SERVER
    SERVER --> AGENT
    AGENT --> MEMORY
    AGENT --> SESSION
    AGENT --> PLAN
    AGENT --> AMBIENT
    AGENT --> FS
    AGENT --> SHELL
    AGENT --> WEB
    AGENT --> INT
    AGENT --> OPENAI
    AGENT --> OPENROUTER
    AGENT --> GEMINI
```

---

# Runtime Philosophy

The runtime is designed around several architectural principles:

## Local-first execution

The backend executes locally on the user's machine.

Benefits include:

- direct filesystem access
- shell execution
- lower latency
- desktop integration
- local orchestration
- privacy-preserving workflows

## Ambient computing model

Instead of isolated chat sessions, iAgent behaves more like:

- an ambient assistant
- a desktop copilot
- a workflow runtime
- an orchestration layer

## Tool-centric design

The LLM is not the system.

The runtime itself is the system.

The architecture prioritizes:

- execution pipelines
- orchestration
- runtime coordination
- planning systems
- tools
- memory
- workflows

---

# Installation

## Prerequisites Check

Before installing, verify your system:

```bash
# Check Rust version
rustc --version

# Check Git version
git --version

# Check PowerShell version
$PSVersionTable.PSVersion
```

## One-Command Install

```powershell
irm "https://raw.githubusercontent.com/benclawbot/iAgent-windows/main/scripts/install.ps1?v=dock" | iex
```

---

# Installed Layout

```text
%LOCALAPPDATA%\\iAgent
├── bin/
├── app/
└── logs/
```

---

# Development

## Build

```bash
cargo build
```

## Release Build

```bash
cargo build --profile release-lto
```

## Run

```bash
cargo run --bin iagent
```

## Run with Environment

```bash
export OPENAI_API_KEY=sk-...
cargo run --bin iagent
```

---

# Testing

## Run All Tests

```bash
cargo test
```

## Run Integration Tests

```bash
cargo test --test integration
```

## Run with Coverage

```bash
cargo tarpaulin --out Xml --out Html
```

## Self-Check Validation

Verify your setup is correct:

```bash
cargo run --bin iagent -- --self-check
```

Expected output:
```
[i] Checking configuration...
[i] Provider: openai ✓
[i] API Key: set ✓
[i] Log directory: accessible ✓
[i] Self-check passed ✓
```

---

# Troubleshooting

## Common Issues

### API Key Not Found

**Error**: `Configuration error: No API key found`

**Solution**: Set your API key before running:
```bash
# Linux/Mac
export OPENAI_API_KEY=sk-...

# Windows PowerShell
$env:OPENAI_API_KEY="sk-..."

# Windows CMD
set OPENAI_API_KEY=sk-...
```

### Provider Connection Failed

**Error**: `Connection failed: timeout reaching provider`

**Solution**: Check your internet connection and API key validity. For proxy users, verify proxy settings.

### Browser Automation Not Working

**Error**: `Browser not found on port 9222`

**Solution**: Launch Chrome with debugging enabled:
```powershell
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222
```

### GitHub Bot Detection

**Error**: `API rate limit exceeded` or `Access blocked`

**Solution**: Configure residential proxy to avoid bot detection:
```bash
export HTTPS_PROXY=http://user:pass@proxyhost:port
```

### Rust Build Errors

**Error**: `linker command not found` or compilation failures

**Solution**: Install Visual Studio Build Tools (Windows) or GCC (Linux/Mac):
```powershell
# Windows: Install via Visual Studio Installer
# Linux
sudo apt install build-essential
```

## Diagnostic Mode

Run with debug logging to diagnose issues:

```bash
RUST_LOG=debug cargo run --bin iagent
```

## Log Files

Find logs at:

| Platform | Log Location |
|----------|-------------|
| Windows | `%LOCALAPPDATA%\iAgent\logs\` |
| Linux | `~/.local/share/iAgent/logs/` |
| macOS | `~/Library/Logs/iAgent/` |

## Get Help

If issues persist:
1. Check existing issues: https://github.com/benclawbot/iAgent-windows/issues
2. Create new issue with log output and system info

---

# Repository Structure

## Runtime

- `src/main.rs` → backend entry point
- `src/agent/*` → execution orchestration
- `src/server/*` → local runtime server
- `src/tool/*` → tool execution layer
- `src/provider/*` → provider routing
- `src/auth/*` → auth and token handling
- `src/ambient/*` → background workflows

---

# Workspace Crates

| Crate | Purpose |
|---|---|
| `jcode-agent-runtime` | Runtime orchestration |
| `jcode-memory-types` | Memory structures |
| `jcode-storage` | Persistence layer |
| `jcode-plan` | Planning engine |
| `jcode-provider-openai` | OpenAI integration |
| `jcode-provider-openrouter` | OpenRouter integration |
| `jcode-provider-gemini` | Gemini integration |
| `iagent-desktop` | Desktop integration |
| `overlay-ui` | Overlay runtime |
| `desktop-monitor` | Desktop monitoring |
| `suggestion-engine` | Suggestion systems |

---

# Long-Term Direction

The architecture is moving toward:

- ambient AI systems
- persistent orchestration
- long-running workflows
- memory-aware agents
- execution-first runtimes
- desktop-native AI environments
- autonomous workflow coordination

This repository is structured more like an operating layer for AI workflows than a traditional chatbot frontend.

---

# Contributing

Areas especially valuable for contribution:

- provider integrations
- tool development
- orchestration systems
- memory systems
- desktop automation
- Windows UX
- runtime reliability
- ambient workflow systems

---

# License

See repository license for details.

---

<div align="center">

### Build agents that don't just chat — but observe, assist, execute, orchestrate, remember, and evolve along with you.

</div>
