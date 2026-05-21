# Product Focus: iAgent Core Loop

## Goal
Ship one high-confidence loop end to end:

`watch context -> suggest action -> user approves -> execute safely -> remember pattern`

## In Scope (Default Product Path)

1. Context capture
- Foreground app/window context from `desktop-monitor`.
- Notification-derived context events with dedupe/throttle.

2. Suggestion generation
- Intent-aware suggestion cards from `suggestion-engine`.
- Confidence-based fallback to rewrite suggestions when intent confidence is low.

3. Safety and approval
- Risk-level policy evaluation in `src/safety.rs`.
- Audit trail entries for queued permissions, decisions, and executed actions.
- Persistent "never again" deny rules.

4. Execution
- Browser execution via CDP (`app-integrations::browser` + `form_fill` workflow).
- Office workflows via `app-integrations::office_workflows`.

5. Learning and memory
- Persist accepted/denied actions and safety decisions for future policy routing.
- Record action transcripts with context and outcomes.

## Out of Scope (Default Path)

1. Autonomous multi-step operation without user checkpoints on high-risk actions.
2. Broad background orchestration across all crates/features by default.
3. Experimental integrations that are not connected to the core loop.

## Runtime Gating Recommendation

1. Mark non-core modules as experimental behind feature flags.
2. Keep core loop crates always-on:
- `desktop-monitor`
- `suggestion-engine`
- `app-integrations`
- `overlay-ui`
- `src/safety.rs` policy flow

3. Route all mutating actions through:
- risk evaluation
- approval decision (if required)
- audit write

## Current Runtime Gates

1. `[features] memory = true|false`
2. `[features] swarm = true|false`
3. `[ambient] enabled = true|false`
4. `[gateway] enabled = true|false`
5. Safety notification channels are opt-in under `[safety]` toggles.

## Operational KPIs

1. Suggestion acceptance rate.
2. Approval latency (p50/p95).
3. Action success rate.
4. Undo/rollback usage rate.
5. Safety blocks by risk level.

## KPI Schema Examples

- `suggestion.accepted`: `{ "intent": "...", "latency_ms": 1234 }`
- `approval.decision`: `{ "approved": true, "latency_ms": 950, "risk_level": "external_send" }`
- `action.execution`: `{ "action_type": "fill_form", "success": true, "risk_level": "edit_local" }`
- `action.undo`: `{ "action_type": "edit", "undo_used": true }`

Events are appended to `~/.jcode/telemetry/core_loop_metrics.jsonl` with `schema_version`.

## Dashboard Query Examples

1. Suggestion acceptance trend by day and intent.
2. Approval latency p50/p95 split by risk level.
3. Action success/failure rate by workflow type.
4. Undo usage share for reversible actions.
