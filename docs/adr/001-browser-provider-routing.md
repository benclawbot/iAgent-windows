# ADR 001: Browser Provider Routing (Firefox Bridge + CDP)

## Status
Accepted - 2026-05-21

## Context
The project now has two browser automation backends:

1. Firefox Agent Bridge (existing runtime integration and operations workflow).
2. CDP-based Chrome/Edge automation (`crates/app-integrations/src/browser.rs`).

We need a stable strategy for routing browser actions without breaking existing Firefox users.

## Decision
Use separate providers behind a common routing layer instead of forcing a single unified provider implementation.

- Keep provider-specific setup/health behavior isolated.
- Route by explicit browser target (`firefox`, `chrome`, `edge`, `auto`).
- Keep the action contract normalized (navigate/interact/fill/evaluate/screenshot/content).
- Preserve backward compatibility by keeping Firefox as the default `auto` behavior until CDP rollout is explicitly enabled in tool routing.

## Consequences

### Positive
- Existing Firefox flows remain stable.
- CDP can evolve independently with retries/timeouts/error taxonomy.
- Failures remain actionable per backend.

### Tradeoffs
- Some behavior remains backend-specific (setup, status, diagnostics).
- Cross-browser parity requires smoke coverage and periodic contract checks.

## Compatibility Notes
- Existing Firefox bridge users are unaffected.
- CDP users can adopt Chrome/Edge incrementally.
- Routing should continue to accept legacy browser inputs and map them to the selected provider.
