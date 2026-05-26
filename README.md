# iAgent Windows

<div align="center">

### Autonomous AI Agent Runtime for Windows

Persistent desktop AI orchestration with local execution, ambient workflows, provider routing, memory systems, and tool-driven automation.
iAgent is a next generation autonomous AI agent platform fully integrated into Windows as an ambient agent providing suggestions and minimally intrusive chat dock to help you accomplish more in your tasks, think co-working and full agentic building/researching activities. It can also interact easily with office Tools (Word, Excel, Powerpoint), web Tools (search for you, fill forms,...). It learns you preferences, evolves thanks to its deep memory layer.
It's always available, has computer use and full agentic capabilities (with swarm agents) but remains in the background for you to focus on what you need to achieve!

![iAgent Infographic](img/1000006999.png)

</div>

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
- asking for explicit approval through floating proposal popups before mutating desktop actions
- coordinating provider-backed reasoning
- integrating directly into Windows UX

The platform combines:

- a modular Rust async runtime
- Python desktop dock, task inbox, and system-wide proposal popups
- backend overlay interfaces
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

OpenAI OAuth uses `http://localhost:1455/auth/callback` as the default local callback URI.

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

## Safety, Approval, and Audit Trail

iAgent's default product loop is:

`watch context -> suggest action -> user approves -> execute safely -> remember pattern`

That loop is backed by runtime safety systems rather than prompt-only promises:

- risk classification for proposed actions
- queued permission requests for actions that mutate the desktop, shell, files, or external systems
- Validate/Refuse proposal popups in the Python dock for commands, typing, and delegated goals
- action history, audit entries, transcripts, summaries, and "never again" rules
- Action Flight Recorder read model and `flight_recorder` tool for inspecting actions, approvals, audit entries, evidence, undo tokens, screenshots, risk/disposition totals, and pending follow-ups
- filters for recorder entries by action text, risk level, disposition, result limit, and optional structured context payloads
- Connector Packs With Permission Scopes for Outlook/Gmail/Calendar, Slack/Teams, GitHub/Linear/Jira, Notion/Obsidian, and file-share style integrations
- connector write preflight decisions that require active write scopes before any external-system mutation, plus a write-evidence ledger that records run ids, tool-call ids, targets, grant ids, required scopes, summaries, and evidence references for every approved connector write
- Proactive Briefings and Next-Best Actions for morning briefings, end-of-task recaps, meeting prep cards, project resume cards, contextual recommendations, and durable "never suggest this again" feedback
- Windows App Intent Manifests through `iagent.intent.json`, letting local apps and scripts declare safe structured actions, parameters, examples, approval levels, and rollback hints for import into iAgent tools and recipe-ready plans
- Remote Dispatch and Watch Mode for authenticated local/remote task submission, mobile-friendly status cards, scheduled jobs, approval-needed notifications, completion evidence, and failure packets
- Attention Budget and Interruption Control for quiet hours, hourly/daily notification caps, snooze/resume, critical-approval bypass, preflight decisions, delivery history, and digest summaries
- Processing Transparency Reports for recording where user data was processed, which processor handled it, what data categories were involved, retention/deletion state, and exportable user-facing audit summaries
- explicit separation between observation actions and mutating actions such as click, type, hotkey, scroll, app launch, and delegated communication

---

## Persistent Memory

Dedicated memory and storage layers enable:

- persistent sessions
- contextual continuity
- structured knowledge
- long-running workflows
- memory-aware orchestration

---

## Personal Desktop Layer

iAgent includes a local-first personal desktop layer for user-approved recall and recovery workflows:

- explicit personal memory through the existing `memory` tool
- smart snippets such as `/sig` for reusable text expansions
- typed snippet expansion support for text that ends with a saved trigger, with optional app scoping
- a Windows global snippet hook in the personal daemon so saved triggers can expand directly in other apps
- contextual reminders tied to the current app/window title, including due/overdue reminder checks
- an always-on personal daemon contract for clipboard capture, app/window snapshots, due reminders, one queued background job, and proactive suggestion events
- a `personal-daemon` CLI that can run once, print status, or stay resident as a headless login daemon
- recent clipboard recovery with duplicate handling, secret redaction, and opt-in capture from the system clipboard
- clipboard pin/delete/clear controls and local retention limits
- active-window capture, recent app/window recall, and Windows focus switching for commands like "switch to the spreadsheet from yesterday"
- background job records plus safe built-in execution for folder summaries and batch-rename previews, with JSON job logs
- searchable Recall-like timeline entries with retention, filters, app exclusions, private-title filtering, and delete controls
- computer-use action planning with observe/act/verify steps, retry-ready verification, permission tiers, and prompt-injection risk flags
- window layout plans, saved named layouts, project workspaces, Windows active-window snapping, and two-window tiling by app/window description
- privacy/settings controls for clipboard history, reminder notifications, background jobs, proactive suggestions, snippet expansion, timeline capture modes, retention, app exclusions, and personal-data clearing
- Sensitive Context Firewall controls for redaction previews, secret/email/payment detection, capture pause/resume, recent-context forgetting, private app/title exclusions, storage counts, retention limits, and screenshot/text capture decisions
- a Settings > Personal panel for daemon status, one-tick runs, daemon start, and opening the local personal-data folder
- UI-ready control-panel summaries for snippets, reminders, clipboard, jobs, privacy, layouts, timeline, and project workspaces

The `personal` tool stores this helper data under the local iAgent/JCode home directory and keeps it separate from durable long-term memory unless the user explicitly asks to remember something.

Run `iagent personal-daemon --headless` to keep the personal layer active in the background, `iagent personal-daemon --once --headless` for a single watcher tick, and `iagent personal-daemon --status` to inspect counts for reminders, jobs, clipboard, timeline, layouts, and project workspaces. The Windows installer creates a hidden `iagent-personal-daemon` Startup shortcut for this daemon unless `-SkipPersonalDaemonSetup` is supplied.

---

## Tool Ecosystem

Integrated tooling includes:

- filesystem read/write/edit/search tools with protected-path handling
- shell and task execution with approval-aware routing
- browser automation through Firefox bridge routing plus Chrome/Edge CDP backends
- form-fill workflows over extracted web form fields
- Office workflows through OfficeCLI-backed Word, Excel, and PowerPoint builders
- `computer` actions for screenshots, active-window context, app listing/opening, clicks, typing, hotkeys, waits, and scrolling
- `personal` actions for snippets, reminders, clipboard recovery, app/window recall, jobs, layouts, and project workspaces
- `personal` Sensitive Context Firewall actions for privacy status, redaction preview, capture pause/resume, and recent-context deletion
- `attention` budget actions for managing quiet hours, interruption caps, snooze/resume, preflighting notifications or suggestions, recording delivery outcomes, and producing attention digests
- `briefing` proactive actions for morning briefings, end-task recaps, meeting prep, project resume, low-noise next-best recommendations, saved recaps, and never-suggest feedback rules
- `connector` pack actions for inspecting built-in connector definitions, granting/revoking explicit read/write scopes, preflighting writes, and auditing recorded write evidence
- `intent` manifest actions for discovering, validating, importing, listing, and planning local `iagent.intent.json` app/script capabilities without turning manifest import into arbitrary-code execution
- `dispatch` watch-mode actions for creating authenticated dispatch clients, submitting local/remote tasks, approving work, checking mobile-friendly status, watching notifications, listing due scheduled tasks, completing with evidence, and attaching failure packets
- `processing_report` actions for recording processing events, filtering/reporting local/private-cloud/external processing, exporting Markdown transparency reports, and marking retained records deleted without losing the audit trail
- `recipe` catalog and command palette actions for searchable, hotkey-ready workflows with typed inputs, approval policies, required tools, and non-executing dispatch plans
- `meeting` memory actions for start/append/finish meeting capture, local speaker/time transcript segments, source-linked decisions/questions/action items, and conversion into reminders, jobs, or delegation drafts
- `flight_recorder` action timeline for user-readable run evidence, approval state, screenshots, undo metadata, and follow-up queues
- `communicate` / swarm delegation tools for assignment-style multi-agent work
- dictation and voice-adjacent runtime support used by the desktop companion
- planning, compaction, memory, MCP registration, self-development, and ambient tool registration layers
- Validate/Refuse proposal popups for AI-suggested commands, typing, and delegated goals

---

## Desktop Integrations

iAgent connects directly to the Windows desktop and key productivity applications through three integration layers.

### Desktop Dock, Companion, and Proposal UX

The tracked Python desktop runtime under `app/iagent-py` provides the Windows-facing user experience:

- tray and dock launcher via `app/launch-iagent.ps1`
- compact companion responses and context capture
- task inbox with running, completed, and feedback-aware task records
- system-wide proposal popups for mutating action tags such as command execution, typing, and delegated backend goals
- Office-goal queuing that routes "make a document/spreadsheet/deck" requests through deterministic builders before opening the result
- settings UI for provider/runtime configuration and personal desktop controls

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
        PROPOSAL[Proposal Popups]
        OVERLAY[Overlay UI]
        INBOX[Task Inbox]
    end

    subgraph Runtime["Rust Runtime Engine"]
        SERVER[Runtime Server]
        AGENT[Agent Executor]
        MEMORY[Memory System]
        PERSONAL[Personal Desktop Layer]
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
    DOCK --> PROPOSAL
    DOCK --> SERVER
    SERVER --> AGENT
    AGENT --> MEMORY
    AGENT --> PERSONAL
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
- `src/safety.rs` → permission queue, audit trail, action history, transcripts, and never-again rules
- `src/personal_layer.rs` and `src/personal_daemon.rs` → snippets, reminders, clipboard recovery, window recall, jobs, layouts, and project workspaces
- `src/dictation.rs` → desktop dictation support
- `src/tool/communicate*` → delegated/swarm communication tooling
- `app/iagent-py/*` → Python dock, task inbox, proposal popups, companion manager, settings UI, and Office goal queue
- `scripts/office/*` → deterministic Office document builders mirrored into the installed runtime

---

# Workspace Crates

| Crate | Purpose |
|---|---|
| `app-integrations` | Browser, form-fill, OfficeCLI, and Office workflow integrations |
| `desktop-monitor` | Active desktop context, window state, and safe file operations |
| `iagent-settings` | Shared desktop/runtime settings |
| `jcode-agent-runtime` | Runtime orchestration |
| `jcode-ambient-types` | Ambient workflow data contracts |
| `jcode-auth-types` | Authentication data contracts |
| `jcode-compaction-core` | Conversation/context compaction support |
| `jcode-memory-types` | Memory structures |
| `jcode-plan` | Planning engine |
| `jcode-protocol` | Runtime protocol and transport types |
| `jcode-provider-core` | Provider abstraction layer |
| `jcode-provider-openai` | OpenAI integration |
| `jcode-provider-openrouter` | OpenRouter integration |
| `jcode-provider-gemini` | Gemini integration |
| `jcode-session-types` | Session state contracts |
| `jcode-storage` | Persistence layer |
| `jcode-tool-core` | Shared tool traits and execution contracts |
| `jcode-tool-types` | Shared tool input/output types |
| `overlay-ui` | Overlay runtime |
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

# Product Roadmap

All final roadmap deliveries identified for this pass are now integrated above as current behavior. Follow-on comparison work also integrated the Attention Budget and Interruption Control layer as current behavior.

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
