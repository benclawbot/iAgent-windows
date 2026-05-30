# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- Installer checksum verification support and uninstall script
- First-run setup wizard for provider + API key bootstrap
- Runtime permissions config (`[permissions]`) and shell execution policy checks
- Memory clear command with explicit confirmation flow
- Skills documentation and example skill package

### Changed
- TUI compatibility builds standardized on `--features tui`
- Logging file naming aligned to `iagent-YYYY-MM-DD.log`
- README and docs synchronized for v1.0 launch criteria

## [0.13.0] - 2026-05-30

### Added
- Binary releases on GitHub
- Structured logging with tracing
- Health check and metrics endpoints
- Configuration file support (`config.toml`)
- Graceful shutdown handling
- Crash recovery with state persistence
- Cargo audit security scanning
- Dependabot for dependency updates

### Changed
- Release profile optimized for better performance
- CI workflow improvements

## [0.12.2] - Initial Professional Release

### Added
- Autonomous AI agent with ambient assistance
- Windows desktop integration
- Office workflow automation
- Terminal UI support
- Multiple AI provider support
- Memory and context management
