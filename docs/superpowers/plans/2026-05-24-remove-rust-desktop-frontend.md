# Remove Rust Desktop Frontend Build Surface

## Goal

Retire the `iagent-desktop` Rust frontend from the active `iagent-windows` workspace so the Python app can be treated as the single user-facing frontend. Keep backend, ambient monitoring, suggestion, overlay, and app-integration crates in place.

## Scope

- Remove `crates/iagent-desktop` from the Cargo workspace.
- Remove self-dev build target routing for `iagent-desktop`.
- Update tests and tool descriptions so builds target the remaining `iagent` binary.
- Remove budget ratchet entries for deleted desktop files.
- Update docs that advertised the Rust desktop binary.
- Delete the `crates/iagent-desktop` source tree.

## Verification

- `cargo fmt --all`
- `cargo test -p iagent test_selfdev_build_command`
- `cargo check --workspace --all-targets --target x86_64-pc-windows-msvc`
- `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings`

## Notes

The crates named `desktop-monitor`, `suggestion-engine`, `overlay-ui`, and `app-integrations` are not removed. They support the ambient backend/suggestion loop and are distinct from the retired Rust frontend crate.
