# Auth Notes: OAuth + API-key Providers

This document explains how authentication works in J-Code.

## Overview

J-Code can detect existing local credentials and can also run built-in OAuth and API-key login flows.

For auth files managed by other tools/CLIs, iagent asks before reading them. If you
approve a source, iagent remembers that approval for that external auth file path
for future sessions and still leaves the original file untouched (no move,
rewrite, or permission mutation). Symlinked external auth files are rejected.

Credentials are stored locally:
- J-Code Claude OAuth (if logged in via `iagent login --provider claude`): `~/.iagent/auth.json`
- Claude Code CLI: `~/.claude/.credentials.json`
- OpenCode (optional provider/OAuth import source): `~/.local/share/opencode/auth.json`
- pi (optional provider/OAuth import source): `~/.pi/agent/auth.json`
- J-Code OpenAI/Codex OAuth: `~/.iagent/openai-auth.json`
- Codex CLI auth source (read in place only after confirmation): `~/.codex/auth.json`
- Gemini native OAuth: `~/.iagent/gemini_oauth.json`
- Gemini CLI import fallback: `~/.gemini/oauth_creds.json`
- Copilot CLI plaintext fallback: `~/.copilot/config.json`
- Legacy Copilot JSON sources: `~/.config/github-copilot/hosts.json`, `~/.config/github-copilot/apps.json`

Relevant code:
- Claude provider: `src/provider/claude.rs`
- OpenAI login + refresh: `src/auth/oauth.rs`
- OpenAI credentials parsing: `src/auth/codex.rs`
- OpenAI requests: `src/provider/openai.rs`
- Azure OpenAI auth/config: `src/auth/azure.rs`
- Azure OpenAI transport: `src/provider/openrouter.rs`
- Gemini login + refresh: `src/auth/gemini.rs`
- Gemini Code Assist provider: `src/provider/gemini.rs`
- OpenAI-compatible provider metadata/login descriptors: `crates/iagent-provider-metadata/src/lib.rs`

## Claude (Claude Max)

### Login steps
1. Run `iagent login --provider claude` (recommended), or `iagent login` and choose Claude.
   - For headless / SSH use: `iagent login --provider claude --no-browser`
   - For scriptable remote flows: `iagent login --provider claude --print-auth-url`, then later complete with `--callback-url` or `--auth-code`
2. Alternative: run `claude` (or `claude setup-token`). iagent can detect `~/.claude/.credentials.json`, ask before reading it, and remember that approval for future sessions.
3. Verify with `iagent --provider claude run "Say hello from iagent"`.

Credential discovery order is:
1. `~/.iagent/auth.json`
2. `~/.claude/.credentials.json`
3. `~/.local/share/opencode/auth.json`
4. `~/.pi/agent/auth.json`

### Direct Anthropic API (default)
`--provider claude` uses the direct Anthropic Messages API by default.
iagent owns the full runtime path itself: auth, refresh, request shaping, tool
compatibility, and transport.

#### Claude OAuth direct API compatibility
Claude Code OAuth tokens can be used directly against the Messages API, but only
if the request matches the Claude Code "OAuth contract". iagent applies this
automatically for the default Claude runtime path.

Required behaviors (applied by the Anthropic provider):
- Use the Messages endpoint with `?beta=true`.
- Send `User-Agent: claude-cli/1.0.0`.
- Send `anthropic-beta: oauth-2025-04-20,claude-code-20250219`.
- Prepend the system blocks with the Claude Code identity line as the first
  block:
  - `You are Claude Code, Anthropic's official CLI for Claude.`

Tool name allow-list:
Claude OAuth requests reject certain tool names. iagent remaps tool names on the
wire and maps them back on responses so native tools continue to work. The
mapping is:
- `bash` → `shell_exec`
- `read` → `file_read`
- `write` → `file_write`
- `edit` → `file_edit`
- `glob` → `file_glob`
- `grep` → `file_grep`
- `task` → `task_runner`
- `todoread` → `todo_read`
- `todowrite` → `todo_write`

Notes:
- If the OAuth token expires, refresh via the Claude OAuth refresh endpoint.
- Without the identity line and allow-listed tool names, the API will reject
  OAuth requests even if the token is otherwise valid.

### Deprecated Claude CLI transport
The old Claude CLI shell-out path is deprecated and should only be used for
legacy compatibility.

You can still force it temporarily with:
- `IAGENT_USE_CLAUDE_CLI=1`
- or `--provider claude-subprocess` (deprecated hidden compatibility value)

These environment variables control the deprecated Claude Code CLI transport:
- `IAGENT_CLAUDE_CLI_PATH` (default: `claude`)
- `IAGENT_CLAUDE_CLI_MODEL` (default: `claude-opus-4-5-20251101`)
- `IAGENT_CLAUDE_CLI_PERMISSION_MODE` (default: `bypassPermissions`)
- `IAGENT_CLAUDE_CLI_PARTIAL` (set to `0` to disable partial streaming)

## OpenAI / Codex OAuth

### Login steps
1. Run `iagent login --provider openai`.
   - For headless / SSH use: `iagent login --provider openai --no-browser`
   - For scriptable remote flows: `iagent login --provider openai --print-auth-url`, then later complete with `--callback-url`
2. Your browser opens to the OpenAI OAuth page unless you use `--no-browser`. The local callback listens on
   `http://localhost:1455/auth/callback` by default.
   If port `1455` is unavailable, iagent falls back to a manual paste flow where
   you can paste the full callback URL or query string.
3. After login, tokens are saved to `~/.iagent/openai-auth.json`.

Credential discovery order is:
1. `~/.iagent/openai-auth.json`
2. `~/.codex/auth.json`
3. trusted OpenCode/pi OAuth in `~/.local/share/opencode/auth.json` / `~/.pi/agent/auth.json`
4. `OPENAI_API_KEY`

If iagent finds existing credentials in `~/.codex/auth.json`, it asks before
reading them. When approved, it remembers that trust decision for future iagent
sessions and still does not move, delete, or rewrite the Codex file.

### Request details
J-Code uses the Responses API. If you have a ChatGPT subscription (refresh
token or id_token present), requests go to:
- `https://chatgpt.com/backend-api/codex/responses`
with headers:
- `originator: codex_cli_rs`
- `chatgpt-account-id: <from token>`

Otherwise it uses:
- `https://api.openai.com/v1/responses`

### Troubleshooting
- Claude 401/auth errors: run `iagent login --provider claude`.
- 401/403: re-run `iagent login --provider openai`.
- Callback issues: make sure port 1455 is free and the browser can reach
  `http://localhost:1455/auth/callback`.

## Azure OpenAI

This was added after comparing J-Code to OpenCode/Crush. The meaningful auth gap
was not another browser OAuth flow, but support for **Azure OpenAI** using either:
- **Microsoft Entra ID** credentials (via Azure's `DefaultAzureCredential` chain), or
- **Azure OpenAI API keys**.

### Login/setup steps
1. Run `iagent login --provider azure`.
2. Enter your Azure OpenAI endpoint, for example:
   - `https://your-resource.openai.azure.com`
3. Enter your Azure deployment/model name.
4. Choose one auth mode:
   - **Entra ID** (recommended)
   - **API key**
5. iagent saves settings to `~/.config/iagent/azure-openai.env`.

### Stored configuration
The Azure env file may contain:
- `AZURE_OPENAI_ENDPOINT`
- `AZURE_OPENAI_MODEL`
- `AZURE_OPENAI_USE_ENTRA`
- `AZURE_OPENAI_API_KEY` (only when using key auth)

### Runtime behavior
- iagent normalizes the endpoint to the newer Azure OpenAI `/openai/v1` base.
- In **Entra ID** mode, iagent obtains bearer tokens using `azure_identity::DefaultAzureCredential` with scope:
  - `https://cognitiveservices.azure.com/.default`
- In **API key** mode, iagent sends the credential in the Azure-style `api-key` header.
- The Azure provider currently reuses J-Code's OpenAI-compatible transport layer under the hood.
- Model catalog fetching is disabled for Azure by default, so you should configure a deployment/model explicitly.

### Entra ID credential sources
`DefaultAzureCredential` can resolve credentials from sources like:
- `az login`
- managed identity
- Azure environment credentials

### Troubleshooting
- If Entra ID auth fails locally, try `az login` first.
- Make sure your identity has access to the Azure OpenAI resource.
- If requests fail with deployment/model errors, verify `AZURE_OPENAI_MODEL` matches your deployed model name.
- If you prefer static credentials, re-run `iagent login --provider azure` and choose API key mode.

## Gemini OAuth

### Login steps
1. Run `iagent login --provider gemini` or `/login gemini` inside the TUI.
   - For headless / SSH use: `iagent login --provider gemini --no-browser`
   - For scriptable remote flows: `iagent login --provider gemini --print-auth-url`, then later complete with `--auth-code`
2. iagent opens a browser to the Google OAuth flow used for Gemini Code Assist unless you use `--no-browser`.
3. If local callback binding is unavailable, iagent falls back to a manual paste flow using `https://codeassist.google.com/authcode`.
4. Tokens are saved to `~/.iagent/gemini_oauth.json`.

### Credential discovery order
1. Native iagent Gemini tokens: `~/.iagent/gemini_oauth.json`
2. Gemini CLI OAuth source (read only after approval): `~/.gemini/oauth_creds.json`
3. trusted OpenCode/pi OAuth in `~/.local/share/opencode/auth.json` / `~/.pi/agent/auth.json`

### Runtime notes
- iagent uses native Google OAuth and talks to the Google Code Assist backend directly.
- Expired tokens are refreshed automatically using the Google refresh token.
- Some school / Workspace accounts may require `GOOGLE_CLOUD_PROJECT` or `GOOGLE_CLOUD_PROJECT_ID` for Code Assist entitlement checks.

### Troubleshooting
- If browser launch fails, use `--no-browser` and the pasted callback/code flow.
- If entitlement or onboarding fails for a Workspace account, set `GOOGLE_CLOUD_PROJECT` and retry.
- If login succeeds but requests fail later, re-run `iagent login --provider gemini` to refresh the stored session.

### Auth verification
Use the built-in auth verifier to test the full local auth/runtime path after login:

```bash
# Run Gemini login now, then verify token refresh + provider smoke
iagent --provider gemini auth-test --login

# Verify existing Gemini auth without re-running login
iagent --provider gemini auth-test

# Check every currently configured supported auth provider
iagent auth-test --all-configured
```

For model providers, `auth-test` attempts:
1. credential discovery
2. refresh/auth probe
3. a real provider smoke prompt expecting `AUTH_TEST_OK`
4. a tool-enabled smoke prompt using the same tool-attached request path as normal chat

Use `--no-tool-smoke` if you only want the auth/simple-runtime checks.

For Gmail/Google it verifies credential discovery and token refresh, but skips model smoke because it is not a model provider.

## OpenAI-compatible API-key providers

J-Code also ships first-class provider presets for many OpenAI-compatible APIs.
These providers use the same built-in login flow pattern: `iagent login --provider <name>`.

For arbitrary OpenAI-compatible APIs, especially when an agent is doing setup, prefer the named profile command instead of hand-editing config:

```bash
printf '%s' "$MY_API_KEY" | iagent provider add my-api \
  --base-url https://llm.example.com/v1 \
  --model my-model-id \
  --api-key-stdin \
  --set-default \
  --json

iagent --provider-profile my-api auth-test --no-tool-smoke
```

This writes `[providers.my-api]` in `~/.iagent/config.toml` and stores the key in iagent's private app config dir, for example `~/.config/iagent/provider-my-api.env`. For localhost servers, use `--no-api-key`.

Two notable presets are:

### Fireworks
- Login: `iagent login --provider fireworks`
- Stored env file: `~/.config/iagent/fireworks.env`
- API key env var: `FIREWORKS_API_KEY`
- Base URL: `https://api.fireworks.ai/inference/v1`
- Default model hint: `accounts/fireworks/routers/kimi-k2p5-turbo`
- Docs: <https://docs.fireworks.ai/tools-sdks/openai-compatibility>

### MiniMax
- Login: `iagent login --provider minimax`
- Stored env file: `~/.config/iagent/minimax.env`
- API key env var: `OPENAI_API_KEY`
- Base URL: `https://api.minimax.io/v1`
- Default model hint: `MiniMax-M2.7`
- Docs: <https://platform.minimax.io/docs/guides/text-generation>

These are first-class iagent provider presets, not just manual custom endpoint examples.
You can still use `openai-compatible` for arbitrary custom providers when there is not a built-in preset.

If iagent finds matching API keys in trusted OpenCode/pi auth files, it can reuse them for the corresponding provider preset without asking you to paste the key again.

## Experimental CLI Providers

J-Code also supports experimental CLI-backed providers, plus Antigravity with native OAuth login:
- `--provider cursor`
- `--provider copilot`
- `--provider antigravity`

Cursor uses iagent's native HTTPS transport. Copilot uses GitHub device-flow auth. Antigravity login/auth storage is handled natively by iagent.

### Cursor
- Login: `iagent login --provider cursor`
  - saves `CURSOR_API_KEY` to `~/.config/iagent/cursor.env`
- Runtime:
  - iagent uses native HTTPS requests
  - if a Cursor API key is configured, iagent exchanges/uses it directly
- Env vars:
  - `IAGENT_CURSOR_MODEL` (default: `composer-1.5`)
  - `CURSOR_API_KEY` (optional; overrides saved key)

### GitHub Copilot
- Login: `iagent login --provider copilot`
  - Headless / SSH: `iagent login --provider copilot --no-browser`
  - Scriptable remote flow: `iagent login --provider copilot --print-auth-url`, then later `iagent login --provider copilot --complete`
  - iagent uses GitHub device code flow and can print the verification URL/QR without opening a local browser.
- Credential discovery order:
  1. `COPILOT_GITHUB_TOKEN`
  2. `GH_TOKEN`
  3. `GITHUB_TOKEN`
  4. trusted `~/.copilot/config.json`
  5. trusted legacy `~/.config/github-copilot/hosts.json`
  6. trusted legacy `~/.config/github-copilot/apps.json`
  7. trusted OpenCode/pi OAuth entries
  8. `gh auth token`
- Env vars:
  - `IAGENT_COPILOT_CLI_PATH` (optional override for CLI path)
  - `IAGENT_COPILOT_MODEL` (default: `claude-sonnet-4`)

### Antigravity
- Login: `iagent login --provider antigravity` (native Google OAuth flow; does **not** require Antigravity to be installed)
  - Headless / SSH: `iagent login --provider antigravity --no-browser`
  - Scriptable remote flow: `iagent login --provider antigravity --print-auth-url`, then later complete with `--callback-url`
- Tokens: `~/.iagent/antigravity_oauth.json`
- Credential discovery order:
  1. native iagent tokens at `~/.iagent/antigravity_oauth.json`
  2. trusted OpenCode/pi OAuth entries when present
- Runtime:
  - iagent authenticates directly and stores/refreshes Antigravity OAuth tokens itself
  - the provider transport still shells out to the Antigravity CLI for completions if you choose `--provider antigravity`
- Env vars:
  - `IAGENT_ANTIGRAVITY_CLIENT_ID` (optional override for OAuth client id)
  - `IAGENT_ANTIGRAVITY_CLIENT_SECRET` (optional override for OAuth client secret)
  - `IAGENT_ANTIGRAVITY_VERSION` (optional override for Antigravity request fingerprint/version)
  - `IAGENT_ANTIGRAVITY_CLI_PATH` (default: `antigravity`, runtime only)
  - `IAGENT_ANTIGRAVITY_MODEL` (default: `default`)
  - `IAGENT_ANTIGRAVITY_PROMPT_FLAG` (default: `-p`)
  - `IAGENT_ANTIGRAVITY_MODEL_FLAG` (default: `--model`)

## Google / Gmail OAuth

### Login steps
1. Run `iagent login --provider google`.
   - For headless / SSH use: `iagent login --provider google --no-browser`
   - For scriptable remote flows after credentials are already configured: `iagent login --provider google --print-auth-url`
2. If Google credentials are not configured yet, iagent first walks you through saving your client ID/client secret or importing the JSON credentials file.
3. For scriptable Google flows, choose the Gmail scope with `--google-access-tier full|readonly` if you do not want the default full access tier.
4. Complete the printed flow later with `iagent login --provider google --callback-url '<full callback url or query>'`.

### Notes
- Google/Gmail scriptable auth requires saved OAuth client credentials first.
- The callback URL can come from a remote browser session that fails on the loopback redirect. Copy the final URL from the address bar and paste or pass it back to iagent.

## Scriptable auth state lifecycle

- iagent stores temporary scriptable login state in `~/.iagent/pending-login/*.json`
- pending state expires automatically
- stale pending entries are cleaned up when scriptable login flows start or resume
- Copilot `--print-auth-url` stores the GitHub device code session and `--complete` resumes polling later
