# Repository Guidelines

## Development Workflow

- Keep changes small and focused.
- Prefer `cargo check`, targeted tests, and dev builds while iterating.
- Build or run the closest available check before pushing.
- Treat this repository as the Windows iAgent product home with a forked
  `jcode` backend, not as the original upstream `jcode` distribution.

## Backend Notes

- The legacy binary and many internal paths are still named `jcode` while the
  extraction is in progress.
- Logs are still written under the legacy `jcode` app-data paths until the
  product naming migration is complete.
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

- Remove or feature-gate the TUI after CI confirms the headless backend build
  boundary.
- Remove unused providers and tools only after the iAgent provider/tool matrix
  is decided.
- Prefer adding narrow API boundaries over more global `pub mod` exposure.
