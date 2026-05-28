# Browser Smoke Suite (Chrome + Edge)

The CDP smoke suite lives in:

- `crates/iagent-app-integrations/tests/browser_smoke.rs`

Both tests are marked `#[ignore]` because they require a live browser launched with remote debugging.

## Local Run

1. Launch Chrome:
`chrome --remote-debugging-port=9222`

2. Launch Edge:
`msedge --remote-debugging-port=9223`

3. Run ignored smoke tests:
`cargo test -p iagent-app-integrations --test browser_smoke -- --ignored`

## Optional Environment Overrides

- `IAGENT_CHROME_CDP_PORT` (default: `9222`)
- `IAGENT_EDGE_CDP_PORT` (default: `9223`)
- `IAGENT_BROWSER_SMOKE_URL` (default: `https://example.com`)
