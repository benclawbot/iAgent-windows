# Remove Rust TUI Build Surface

## Goal

Retire the Rust terminal UI from `iagent-windows` now that the Python app owns the user-facing frontend. Keep the backend CLI/server, ambient desktop backend, suggestion engine, overlay runtime, and Python app integration paths.

## Cross-Reference Findings

- TUI workspace crates: `iagent-tui-account-picker`, `iagent-tui-core`, `iagent-tui-markdown`, `iagent-tui-mermaid`, `iagent-tui-messages`, `iagent-tui-render`, `iagent-tui-session-picker`, `iagent-tui-style`, `iagent-tui-tool-display`, `iagent-tui-usage-overlay`, and `iagent-tui-workspace`.
- Root TUI source: `src/tui*`, `src/cli/tui_launch.rs`, `src/cli/terminal.rs`, `src/bin/tui_bench.rs`, and `src/video_export.rs`.
- Cargo wiring: optional TUI deps plus the retired TUI feature wiring.
- CI wiring: the existing Windows Backend `Check terminal-ui build` job remains because the current GitHub token cannot push workflow edits; `terminal-ui` is retained as a no-op compatibility feature.
- Cleanup targets: docs, dependency-boundary script, size/error/test budget ratchets, and TUI-only command variants.

## Execution

- Remove the TUI crates and root TUI source files.
- Remove TUI dependencies and keep `terminal-ui` as a no-op compatibility feature until workflow-scope credentials can rename/remove the CI job.
- Replace default/no-subcommand TUI launch behavior with a clear backend CLI error.
- Remove or disable TUI-only command paths such as permissions UI, transcript injection, replay playback, and visible ambient cycles.
- Regenerate `Cargo.lock`.

## Verification

- `cargo fmt --all`
- `cargo generate-lockfile`
- `cargo check --workspace --all-targets --target x86_64-pc-windows-msvc`
- Linux/WSL `cargo clippy --workspace --all-targets -- -D warnings`
- Targeted CLI/selfdev tests
- Dependency boundary script
