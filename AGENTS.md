# Repository Guidelines

## Development Workflow

- Keep changes small and focused.
- Prefer `cargo check`, targeted tests, and dev builds while iterating.
- Build or run the closest available check before pushing.
- Treat this repository as the Windows iAgent product home with a forked
  `iagent` backend, not as the original upstream `iagent` distribution.

## Backend Notes

- Product-facing naming is `iAgent` and new user-facing paths/docs must use
  `iAgent` terminology consistently.
- Legacy `iagent` compatibility aliases may still exist for backward
  compatibility, but should not be introduced in new user-facing behavior.
- `scripts/dev_cargo.sh` is kept because self-dev backend helpers still refer to
  it. Other legacy benchmark, demo, Linux release, and remote-build scripts have
  been removed from this snapshot.

## Windows Notes

- `scripts/install.ps1` is the remaining installer script while Windows
  packaging is redesigned.
- `%LOCALAPPDATA%\iAgent\bin\iagent.exe` and related `%LOCALAPPDATA%\iAgent`
  paths are the canonical Windows runtime layout.

## Cleanup Direction

- Keep the retired Rust TUI out of the active build surface; the Python app is
  the user-facing frontend.
- Remove unused providers and tools only after the iAgent provider/tool matrix
  is decided.
- Prefer adding narrow API boundaries over more global `pub mod` exposure.
