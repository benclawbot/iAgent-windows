# Agent-S Integration Contract

iAgent integrates Agent-S (<https://github.com/simular-ai/Agent-S>) as a
planning and grounding pattern, not as an unrestricted desktop automation
runtime.

## Boundary

- Agent-S-compatible planners may propose ACI-style actions only.
- iAgent executes desktop actions through the Rust `computer` tool.
- The `computer` tool schema is the compatibility surface:
  `screenshot`, `click`, `type`, `hotkey`, `scroll`, `wait`,
  `active_window`, and `context`.
- Shell, Python, and arbitrary code execution are not valid paths for ordinary
  desktop mouse, keyboard, screenshot, or scroll control.

## Safety Policy

Observation actions are allowed directly:

- `screenshot`
- `active_window`
- `context`
- `wait`

Desktop-mutating actions require the iAgent permission boundary during an
agent turn and must not be executed until approved:

- `click`
- `type`
- `hotkey`
- `scroll`

Direct mode remains available for explicitly invoked local tooling and focused
tests, where the caller is already outside an autonomous agent turn.

## Sidecar Compatibility

A future Agent-S sidecar may translate UI state into proposed tool calls, but
the sidecar must return structured `computer` actions instead of invoking
Agent-S controller paths that execute generated Python. iAgent remains the
executor, audit point, and permission authority.
