# iAgent Windows

<div align="center">

### Autonomous AI Agent Runtime for Windows

Persistent desktop AI orchestration with local execution, ambient workflows, provider routing, memory systems, and tool-driven automation.
iAgent is a next generation autonomous AI agent platform fully integrated into Windows as an ambient agent providing suggestions and minimally intrusive chat dock to help you accomplish more in your tasks, think co-working and full agentic building/researching activities. It can also interact easily with office Tools (Word, Excel, Powerpoint), web Tools (search for you, fill forms,...). It learns you preferences, evolves thanks to its deep memory layer.
It's always available, has computer use and full agentic capabilities (with swarm agents) but remains in the background for you to focus on what you need to achieve!

![iAgent Infographic](docs/assets/iagent-infographic.jpg)

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
| `jcode-desktop` | Desktop integration |
| `overlay-ui` | Overlay runtime |
| `desktop-monitor` | Desktop monitoring |
| `suggestion-engine` | Suggestion systems |

---

# Installation

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

### Build agents that don't just chat — but execute, orchestrate, remember, and evolve.

</div>
