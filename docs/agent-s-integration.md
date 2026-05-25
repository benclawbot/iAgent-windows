# Agent-S Integration Contract

iAgent integrates Agent-S (<https://github.com/simular-ai/Agent-S>) as a
planning and grounding pattern, not as an unrestricted desktop automation
runtime.

## Boundary

- Agent-S-compatible planners may propose ACI-style actions only.
- iAgent executes desktop actions through the Rust `computer` tool.
- The `computer` tool schema is the compatibility surface:
  `screenshot`, `click`, `type`, `hotkey`, `scroll`, `wait`,
  `active_window`, `context`, `open_app`, and `list_apps`.
- Shell, Python, and arbitrary code execution are not valid paths for ordinary
  desktop app launching, mouse, keyboard, screenshot, or scroll control.

## Safety Policy

Observation and constrained app-discovery actions are allowed directly:

- `screenshot`
- `active_window`
- `context`
- `wait`
- `list_apps`
- `open_app`

Desktop-mutating actions require the iAgent permission boundary during an
agent turn and must not be executed until approved:

- `click`
- `type`
- `hotkey`
- `scroll`

`open_app` is deliberately constrained to installed app entries discovered from
the user's Desktop, public Desktop, OneDrive Desktop, and Start Menu folders.
It uses the native Windows `ShellExecuteW` API for the matched shortcut or
executable; it is not a general command runner and must not be used for URLs or
generated commands.

Direct mode remains available for explicitly invoked local tooling and focused
tests, where the caller is already outside an autonomous agent turn.

## Sidecar Compatibility

A future Agent-S sidecar may translate UI state into proposed tool calls, but
the sidecar must return structured `computer` actions instead of invoking
Agent-S controller paths that execute generated Python. iAgent remains the
executor, audit point, and permission authority.
