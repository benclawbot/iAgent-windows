# iAgent Windows

Ambient agent backend and Windows runtime for iAgent.

This repository contains the Rust runtime that powers iAgent on Windows,
including the CLI/server process, ambient automation loop, tool execution,
provider routing, auth flows, memory, and local integrations.

## Architecture (Current)

The codebase is organized as a single Rust workspace with one primary library
crate (`iagent`) plus several supporting internal crates.

High-level flow:

1. Process startup enters `src/main.rs` and calls `iagent::run()`.
2. CLI startup (`src/cli/startup.rs`) selects mode and initializes config.
3. The server runtime handles sessions, tools, providers, background jobs, and
   streaming responses.
4. Optional ambient and UI binaries run specialized loops on top of the same
   core modules.

### Runtime Entry Points

Defined in `Cargo.toml`:

- `iagent` -> `src/main.rs`
- `iagent-ambient` -> `src/bin/ambient.rs`
- `iagent-overlay-ui` -> `src/bin/overlay_ui.rs`
- `iagent-test-api` -> `src/bin/test_api.rs`
- `iagent-harness` -> `src/bin/harness.rs`

### Core Subsystems

- `src/cli/*`: command parsing, startup orchestration, terminal launch, login
  and provider initialization flows.
- `src/server/*`: local runtime server, client/session lifecycle, reload,
  background tasks, state management, and runtime diagnostics.
- `src/agent/*`: turn execution loop, prompting, streaming, tool-call handling,
  response recovery, and compaction integration.
- `src/tool/*`: tool registry and tool implementations (filesystem, shell,
  web/browser, todo, memory, and integration tools).
- `src/provider/*` + `src/provider_catalog*`: model/provider routing and
  provider-specific behavior.
- `src/auth/*`: auth state, login flows, token refresh, external auth sources,
  and provider auth integrations.
- `src/ambient/*`, `src/ambient_runner.rs`, `src/ambient_scheduler.rs`:
  ambient loop state, scheduling, directives, and persistence.
- `src/memory*`: memory graph, memory logs, caching/pending state, and memory
  agent coordination.
- `src/mcp/*`: MCP client/manager and shared MCP server pool integration.
- `src/transport/*` + `src/protocol*`: local transport and protocol plumbing
  between clients and the runtime server.

### Integration/Support Crates

The workspace includes internal crates for provider metadata, protocol/types,
UI layers, update logic, desktop helpers, and integration adapters. Some crate
directories still use legacy path naming; public/runtime naming is `iagent`.

## Build Profiles and Features

Current feature setup in `Cargo.toml`:

- Default feature set: `pdf`
- Optional: `terminal-ui`
- Optional: `embeddings`
- Optional allocator tuning: `jemalloc`, `jemalloc-prof`

`src/main.rs` also configures allocator/runtime behavior for long-running
process stability and memory behavior under bursty workloads.

## CI (Current)

GitHub Actions workflow: `.github/workflows/windows-backend.yml`

On Windows runners, CI currently runs:

1. PowerShell syntax validation: `./scripts/check_powershell_syntax.ps1`
2. `cargo check --workspace --all-targets` (default features)
3. `cargo check --workspace --all-targets --no-default-features --features pdf`
4. `cargo check --workspace --all-targets --features terminal-ui`

This matrix keeps headless and terminal-ui configurations validated in parallel.

## Development Notes

- Primary library entry: `src/lib.rs`
- Main startup handoff: `iagent::run()`
- Ambient daemon entry: `src/bin/ambient.rs`
- Overlay daemon entry: `src/bin/overlay_ui.rs`
- Server runtime implementation: `src/server/*`

For architecture changes, keep runtime boundaries explicit:

- CLI/startup boundary (`src/cli/*`)
- Server/session boundary (`src/server/*`)
- Agent/tool boundary (`src/agent/*`, `src/tool/*`)
- Provider/auth boundary (`src/provider/*`, `src/auth/*`)
