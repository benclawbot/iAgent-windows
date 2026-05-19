# iAgent Windows

Ambient agent for Windows.

This repository is the Windows-focused home for iAgent. The product layer should
own the Windows experience: background presence, tray integration, hotkeys,
notifications, packaging, update flow, local permissions, and user-facing task
state.

The Rust backend in this repository is a stripped fork of `jcode` used as the
task engine. It provides provider routing, tool execution, sessions, background
tasks, memory, and orchestration primitives that iAgent can call from the
Windows shell.

## Current Snapshot

This repo started from the larger `jcode` fork and has had the first layer of
non-iAgent clutter removed:

- demo videos, screenshots, and README assets
- embedded plugin/runtime experiments
- mobile/iOS prototypes and simulator crates
- Linux desktop packaging
- Figma/mockup artifacts
- telemetry worker deployment code
- duplicate compatibility files that are no longer referenced
- old heavyweight GitHub release workflows

The remaining Rust workspace is intentionally conservative. It keeps the backend
pieces that may still be required until the Windows app boundary is finalized
and verified with a real build.

## Intended Shape

The target architecture is:

- `iAgent Windows`: product shell, Windows integration, permissions, lifecycle,
  install/update, and ambient UX.
- `jcode backend`: internal task engine for providers, tools, sessions,
  orchestration, memory, and background execution.
- A narrow local API between them, likely process/socket based at first, with a
  typed protocol once the app workflows settle.

## Next Cleanup Passes

1. Identify the exact backend entrypoints used by the Windows app.
2. Move the backend behind a smaller crate/API boundary.
3. Feature-gate or remove the terminal TUI if iAgent does not need it.
4. Remove unused providers and tools after the product provider matrix is set.
5. Replace old `jcode` install/release scripts with Windows-first packaging.
6. Add CI that checks the backend and Windows-facing integration without running
   legacy release jobs.

## Development

The legacy binary is still named `jcode` while the extraction is in progress.
That is expected for now. Rename user-facing binaries and package metadata only
after the backend boundary is stable.
