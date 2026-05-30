# Configuration Reference

`iagent-windows` uses `config.toml` as the canonical runtime configuration file.

- Primary path: `%LOCALAPPDATA%\\iAgent\\config.toml` (Windows launcher/runtime)
- Cross-platform/default runtime path: `~/.iagent/config.toml`
- Environment variables can override config values at startup.

## Core Sections

### `[provider]`

- `default_provider` (`string`, optional): provider id to prefer on startup.
- `default_model` (`string`, optional): model id to prefer on startup.
- `openai_reasoning_effort` (`string`, default `"low"`): `none|low|medium|high|xhigh`.
- `openai_transport` (`string`, optional): `auto|websocket|https`.
- `openai_service_tier` (`string`, default `"priority"`): `priority|flex`.
- `cross_provider_failover` (`string`, default `"countdown"`): `countdown|manual`.
- `same_provider_account_failover` (`bool`, default `true`).

### `[features]`

- `memory` (`bool`, default `true`)
- `swarm` (`bool`, default `true`)
- `message_timestamps` (`bool`, default `true`)
- `update_channel` (`string`, default `"stable"`): `stable|main`

### `[permissions]`

- `shell_execution` (`string`, default `"proposal"`): `proposal|auto|disabled`
- `file_write_paths` (`array<string>`, default `["~", "%USERPROFILE%\\Projects"]`)
- `network_access` (`bool`, default `true`)
- `elevation_allowed` (`bool`, default `false`)

### `[safety]`

Notification and approval transport settings (desktop, ntfy, email, Telegram, Discord).

### `[ambient]`

Ambient scheduling, provider overrides, and desktop monitoring/suggestion controls.

## Example

```toml
[provider]
default_provider = "openai"
openai_reasoning_effort = "low"

[features]
memory = true
swarm = true
update_channel = "stable"

[permissions]
shell_execution = "proposal"
file_write_paths = ["~", "%USERPROFILE%\\Projects"]
network_access = true
elevation_allowed = false
```
