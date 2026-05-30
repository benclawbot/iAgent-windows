# Tool and Provider Matrix (v1.0)

This document defines what is considered production-ready for `iagent-windows` v1.0.

## Provider Support

### Shipped and supported

- `openai`: primary provider path for direct API-key and OAuth-backed usage.
- `openrouter`: primary multi-model routing provider.
- `gemini`: supported via configured credentials (API key or trusted external OAuth source).

### Feature-gated / optional

- `bedrock`: AWS provider path. Build with `--features bedrock` to compile the
  AWS SDK dependencies and enable the provider. Default builds exclude Bedrock.

## Canonical Tool Interface

All first-party tools implement the shared trait in:

- `crates/iagent-tool-core/src/lib.rs` (`Tool`, `ToolContext`, `ToolOutput`)

The runtime registers these tool implementations in:

- `src/tool/mod.rs`

This is the contract used by provider bridges and agent runtime orchestration.

## Tool Packaging Categories

### Core shipped tools

Core tools are enabled in the default runtime registry and maintained in this repo, including:

- filesystem/code tools: `read`, `write`, `edit`, `multiedit`, `patch`, `apply_patch`, `ls`, `glob`, `grep`, `file`
- execution tools: `bash`, `batch`, `bg`
- desktop/browser tools: `computer`, `browser`, `app`, `word`
- orchestration tools: `goal`, `todo`, `task`, `dispatch`, `communicate`, `swarm`
- memory/session tools: `memory`, `conversation_search`, `session_search`
- integrations and workflow tools: `connector`, `gmail`, `meeting`, `briefing`, `attention`, `recipe`, `processing_report`, `intent`, `personal`, `flight_recorder`

### Optional / environment-dependent

- tools requiring external binaries, credentials, or service setup remain available but may degrade gracefully with actionable errors.
- browser automation requires an enabled Chrome/Edge CDP endpoint or running browser setup flow.

### Community / contributed

- skill-authored workflows are loaded from skill directories and can orchestrate first-party tools without code changes.
