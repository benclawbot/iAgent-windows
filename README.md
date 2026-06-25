# iAgent Windows

Desktop-native AI agent runtime for Windows with a Python dock shell, a Rust backend runtime, local tool execution, explicit safety controls, ambient desktop assistance, and persistent memory.

## Minimum Requirements

- Windows: **Windows 10 22H2+** (Windows 11 recommended)
- RAM: **8 GB minimum** (16 GB recommended for large sessions)
- Disk: **2 GB free** minimum (more for build/test workflows)
- Tooling for source builds:
  - Rust **1.70+** (`rustc --version`)
  - Git (`git --version`)
  - PowerShell **5.1+** (`$PSVersionTable.PSVersion`)

## Install (Windows)

Pinned script URL (recommended):

```powershell
irm "https://raw.githubusercontent.com/benclawbot/iAgent-windows/main/scripts/install.ps1?v=0.13.0" | iex
```

The installer performs SHA256 verification against release `checksums.txt` before installing downloaded binaries.

Useful switches:

- `-SkipDockSetup`
- `-SkipHotkeySetup`
- `-SkipPersonalDaemonSetup`
- `-SkipDesktopShortcut`
- `-SkipAlacrittySetup`

Uninstall:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\uninstall.ps1
```

## Verified Architecture

The diagram below is intentionally limited to components that exist in this repository. It does not claim services, agents, data stores, or integrations that are not present in code or docs.

<p align="center">
  <img src="https://raw.githubusercontent.com/benclawbot/iAgent-windows/main/assets/iagent-architecture.svg" alt="iAgent Windows verified architecture diagram" width="100%">
</p>

### Component map

| Layer | Verified components | Source paths |
| --- | --- | --- |
| Installer and launchers | Release download, checksum verification, binary install, dock setup, `uv` setup, optional worker dependencies, Alacritty setup, desktop shortcut, Alt+; hotkey launcher, personal daemon launcher | `scripts/install.ps1`, `app/launch-iagent.ps1` |
| Python desktop shell | PySide6 tray app, history window, task inbox, prompt dock, proposal popups, hotkey monitor, mic capture, screen capture, background command runner, companion manager, first-run config bootstrap | `app/iagent-py/iagent/app.py`, `app/iagent-py/pyproject.toml` |
| Rust backend entrypoints | `iagent`, `iagent-test-api`, `iagent-harness`, `iagent-ambient`, `iagent-overlay-ui` binaries | `Cargo.toml`, `src/main.rs`, `src/bin/*` |
| CLI and server runtime | Startup profile, logging, config permission hardening, telemetry checks, update checks, provider initialization, server mode, one-shot `run`, login, auth, provider, model, memory, session, ambient, personal daemon, browser setup, restart commands | `src/cli/startup.rs`, `src/cli/args.rs`, `src/cli/dispatch.rs`, `src/server.rs` |
| Agent session core | Provider-backed agent, tool registry, skill registry, session state, active skill restrictions, memory enablement flag, soft interrupts, background-tool signal, graceful shutdown signal, cache tracking, usage tracking | `src/agent.rs` |
| Tool registry | File/code tools, shell tools, browser/computer/app/Word tools, Gmail/meeting/briefing tools, memory/session/conversation search, subagent/batch/swarm orchestration, skills, MCP, self-development, scheduling | `src/tool/mod.rs`, `crates/iagent-tool-core`, `crates/iagent-tool-types` |
| Providers | Multi-provider layer with OpenAI, OpenRouter, Gemini, Copilot, Cursor, Claude, Anthropic, Antigravity, iAgent provider path, and feature-gated Bedrock | `src/provider/mod.rs`, `crates/iagent-provider-*` |
| Ambient desktop mode | Desktop monitor, notification detector, importance scoring, suggestion engine, overlay daemon, app filters, provider adapter, safety handle, ambient queue/state/scheduling | `src/desktop_ambient.rs`, `src/ambient.rs`, `src/ambient/runner.rs`, `crates/desktop-monitor`, `crates/suggestion-engine`, `crates/overlay-ui` |
| Persistence and configuration | Windows config, cross-platform config, logs, sessions, JSON memory graph files, ambient queue/state, backup recovery for memory graph | `docs/configuration.md`, `docs/memory.md`, `src/storage.rs`, `crates/iagent-storage` |

## Runtime Flow

1. `scripts/install.ps1` installs the Windows binary and, unless skipped, installs the desktop dock app under `%LOCALAPPDATA%\iAgent\app`.
2. `app/launch-iagent.ps1` starts the Python dock with `uv run python -m iagent` and starts the optional worker only when `worker_url` is configured.
3. The Python app starts the tray shell, loads `%APPDATA%\iAgent\config.toml`, wires the hotkey, mic/screen capture, task inbox, history, proposal popups, and companion manager.
4. User prompts or queued goals are sent to the Rust backend through `iagent run --json --quiet <goal>` or through the long-running server path.
5. The Rust runtime initializes providers, sessions, tools, skills, memory, and safety controls before executing agent turns.
6. Mutating shell/file/browser/Office actions remain proposal-controlled unless explicitly configured otherwise.

## Trust & Safety

Mutating actions are designed to be explicit and auditable.

### Proposal flow

Actions that can change local state are expected to go through approval controls, including:

- shell execution
- file writes/deletes
- desktop/browser form submission
- Office document mutations

When a proposal is shown, users can approve or reject.

- **Reject**: action is cancelled and logged.
- **Approve**: action executes and is logged with timestamp.
- On restart after a crash, pending proposals are never auto-executed.

### Auto-approve mode

Power users can run unattended mode with:

```powershell
iagent --auto-approve
```

This bypasses interactive approval prompts for proposal-mode shell actions. Use only in trusted environments.

### Permissions policy (`config.toml`)

`[permissions]` controls shell behavior and scope:

```toml
[permissions]
shell_execution = "proposal"        # proposal | auto | disabled
file_write_paths = ["~", "%USERPROFILE%\\Projects"]
network_access = true
elevation_allowed = false
```

Shell execution audit entries are appended to `shell-audit.jsonl` under the iAgent logs directory.

### Antivirus-friendly build boundary

iAgent keeps desktop automation capabilities available in runtime builds, but default Windows test runs avoid Cargo's monolithic library unit-test executable. That harness can look ransomware-like to behavior engines because one constantly changing unsigned process rapidly exercises memory, file, network, provider, desktop, and background-job code. Maintainers can still run the full library unit sweep explicitly with `cargo test --lib` in a trusted build environment.

Library-test builds also use non-executing desktop stubs, a repo-local temp directory, and stripped symbols to reduce false-positive surface. Windows builds embed iAgent version metadata and an `asInvoker` application manifest. Release artifacts should still be code-signed by the distribution pipeline; unsigned local debug and test binaries can trigger reputation or behavior-based antivirus heuristics.

## Browser Automation Setup

Browser control uses Chrome/Edge DevTools Protocol (CDP).

If no debuggable browser is available, choose one of:

1. launch a managed browser instance with debugging enabled
2. update browser launch configuration to include `--remote-debugging-port=9222`

See `docs/browser-smoke.md` for smoke-test commands.

## Headless Build Boundary

Default builds are headless. TUI compatibility checks are feature-gated.

- Headless default: `cargo build`
- TUI compatibility feature: `cargo build --features tui`

The Rust TUI is not the primary end-user interface; the Python dock app is the user-facing shell.

## Provider Matrix (v1.0)

Shipped and supported:

- OpenAI
- OpenRouter
- Gemini

Optional feature-gated provider path:

- AWS Bedrock (`cargo build --features bedrock`)

See `docs/tools.md` for details.

## First Run

If no config exists, interactive launch triggers a setup wizard that:

1. prompts for provider choice (OpenAI/OpenRouter/Gemini)
2. prompts for API key
3. writes `config.toml`
4. runs a self-check summary

## Configuration and Docs

- Config reference: `docs/configuration.md`
- Tool/provider matrix: `docs/tools.md`
- Skills authoring/discovery: `docs/skills.md`
- Memory durability/export/clear: `docs/memory.md`
- Product naming conventions: `docs/product-naming.md`
- OAuth/auth notes: `OAUTH.md` (OpenAI browser login uses `http://localhost:1455/auth/callback` by default)
- Telemetry/privacy: `TELEMETRY.md`

## Development

```powershell
cargo check --workspace --all-targets
cargo test --workspace --tests
cargo clippy --workspace --all-targets -- -D warnings
```

CI gates include Windows builds, clippy warnings-as-errors, focused tests, and coverage generation.

## Contributing

Contribution workflow is documented in `CONTRIBUTING.md`.

## License

This project is licensed under the **MIT License**. See `LICENSE`.

## Suggested GitHub Topics

- `windows`
- `ai-agent`
- `rust`
- `ambient-computing`
- `llm`
- `automation`
