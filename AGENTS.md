# Repository Guidelines

## Development Workflow

- Keep changes small and focused.
- Prefer `cargo check`, targeted tests, and dev builds while iterating.
- Build or run the closest available check before pushing.
- Treat this repository as the Windows iAgent product home with a forked
  `jcode` backend, not as the original upstream `jcode` distribution.

## Backend Notes

- The public binary and user-facing app-data paths are `iagent`/`iAgent`.
  Runtime data and logs resolve under `%LOCALAPPDATA%\iAgent` on Windows.
- Runtime/provider crates that still carry upstream implementation coupling keep
  their `jcode-*` names until the API boundary is stable.
- `scripts/dev_cargo.sh` is kept because self-dev backend helpers still refer to
  it. Other legacy benchmark, demo, Linux release, and remote-build scripts have
  been removed from this snapshot.

## Windows Notes

- `scripts/install.ps1` is the remaining installer script while Windows
  packaging is redesigned.
- `%LOCALAPPDATA%\iAgent\bin\iagent.exe` and related `%LOCALAPPDATA%\iAgent`
  paths are transitional backend paths.
- New user-facing packaging should target iAgent naming once the backend API
  boundary is stable.

## Cleanup Direction

- Keep the retired Rust TUI out of the active build surface; the Python app is
  the user-facing frontend.
- The `terminal-ui` Cargo feature is a compatibility no-op for old CI and caller
  scripts. Do not reintroduce Rust TUI code behind it.
- AWS Bedrock support is behind the opt-in `bedrock` Cargo feature. Default
  builds should not compile the AWS SDK crates.
- Remove unused providers and tools only after the iAgent provider/tool matrix
  is decided.
- Prefer adding narrow API boundaries over more global `pub mod` exposure.

## Naming Migration Status

| Phase | Crates | Status |
|-------|--------|--------|
| 1 - UI/integration surface | `iagent-overlay-ui`, `iagent-desktop-monitor`, `iagent-suggestion-engine`, `iagent-app-integrations` | Done |
| 2 - Type/data contracts | `iagent-*-types` | Done |
| 3 - Runtime/logic crates | `jcode-agent-runtime`, `jcode-provider-*`, `jcode-tool-core`, `jcode-storage`, `jcode-plan`, `jcode-swarm-core`, `jcode-protocol`, `jcode-core`, and other upstream-coupled crates | Pending API stabilization |

Phase 3 crates keep their `jcode-*` names until their public contracts are
stable enough to distinguish iAgent-owned APIs from upstream-derived runtime
logic.

## Platform Notes

The hotkey subsystem uses platform-native bindings:

- Windows: Win32 hotkey helpers through `windows-sys` and installer-generated
  PowerShell launchers.
- macOS: the `global-hotkey` crate, gated to `target_os = "macos"`.

iAgent-windows ships Windows-first. The macOS dependency is retained only to keep
the gated developer build path compiling on macOS; it has no Windows product
surface.

## Vendored Dependencies

`agentgrep` is vendored as the local workspace crate `crates/agentgrep` and must
stay sourced from that path. Do not restore the old external git dependency on
`https://github.com/1jehuang/agentgrep.git`.

## Dock Runtime

The dock frontend is currently Python-based under `app/iagent-py`. Distribution
should prefer a self-contained PyInstaller artifact so end users do not need a
system Python installation. Long term, migrate the dock into the Rust/WebView2
surface (`wry` plus the overlay UI crate) so the Python runtime can be removed.
