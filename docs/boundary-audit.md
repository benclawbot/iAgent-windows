# Boundary Audit — §4.1 / §4

**Date**: 2026-05-28
**Status**: Complete

## Entrypoint: `iagent-ambient` (`src/bin/ambient.rs`)

```rust
iagent::desktop_ambient::run(false).await
```

## Current Architecture

All Windows shell components (`desktop-monitor`, `suggestion-engine`, `overlay-ui`) and the jcode backend compile into the **same process**. No IPC boundary yet. The `iagent-ambient` binary is the ambient agent — it owns the iAgent provider adapter and drives the event loop.

---

## Key Integration Points Crossing the Future Boundary

### 1. `desktop_ambient.rs` — Ambient Agent Entrypoint

This is where the Windows shell and backend are currently woven together in-process:

```
desktop_ambient::run()
  ├─ DesktopMonitor::new()              [crate: desktop-monitor]
  ├─ monitor.start_monitoring()         → mpsc::Receiver<WindowContext>
  ├─ IAgentProviderAdapter { provider }  ← bridges backend Provider trait to LanguageModelProvider
  ├─ SuggestionEngine::new()             [crate: suggestion-engine]
  ├─ spawn_overlay_daemon()             [crate: overlay-ui]
  └─ tokio::select! loop
       ├─ context_rx.recv()             <- WindowContext from desktop-monitor
       │   └─ engine.generate_suggestions(text, &context)
       │       └─ overlay_client.show_suggestions(suggestions, cursor_position)
       └─ notification_rx.recv()        <- from NotificationDetector
           └─ overlay_client.show_notification()
```

**Key types crossing boundary:**
- `WindowContext` (desktop-monitor → suggestion-engine) — app name, window title, context type, text content, cursor position
- `Suggestion`, `SuggestionIntent`, `ActionCard` (suggestion-engine → overlay-ui) — suggestion data
- `NotificationDetector` state events (desktop-monitor → overlay-ui via desktop_ambient)
- `LanguageModelProvider` trait (suggestion-engine → crate:iagent provider) — simple `complete(prompt) -> String`

---

## Current IPC Communication (in-process, no serialization)

All cross-boundary communication is **in-memory Rust structs** via `mpsc` channels:

| From | To | Channel | Data |
|---|---|---|---|
| `DesktopMonitor` | `desktop_ambient` | `mpsc::Receiver<WindowContext>` | `WindowContext` struct |
| `NotificationDetector` | `desktop_ambient` | `mpsc::Receiver<NotificationEvent>` | `NotificationState` + importance |
| `suggestion-engine` | `overlay-ui` | `mpsc::UnboundedSender<OverlayEvent>` | `OverlayEvent` enum |

---

## The `LanguageModelProvider` Trait — Key Abstraction

```rust
#[async_trait]
pub trait LanguageModelProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
}
```

`IAgentProviderAdapter` wraps the backend `Arc<dyn Provider>` and implements this trait, bridging from the typed backend provider to the suggestion engine.

**Observation**: This trait is the cleanest seam in the system — it's a simple async fn, no protocol complexity, no serialization.

---

## Types That Should Cross the IPC Boundary (for v1)

### Request types (Shell → Backend)
- `SuggestionRequest { text: String, window_context: WindowContext }` — when user is typing
- `NotificationIntercept { app: String, title: String, preview: String }`
- `GetActiveWindow` → `WindowContext`
- `ConfigGet` / `ConfigSet`
- `Shutdown` / `ReloadProvider`

### Response types (Backend → Shell)
- `SuggestionResponse { suggestions: Vec<Suggestion> }`
- `NotificationResponse { important: bool, score: f32 }`
- `ConfigValue { key: String, value: SerdeJsonValue }`
- `HealthCheck { backend_version: String, providers_ready: Vec<String> }`

### Session management
- `SessionId: Uuid` — a `session_id` that identifies each ambient session
- Backend process lifecycle (spawn backend on session start, kill on session end)

---

## Recommended IPC Strategy

### Phase 1: Named Pipe (Minimal Change)
Use Windows named pipes (`\\.\pipe\iagent-backend-{session_id}`) for the IPC transport between a spawned backend process and the shell. Already in tokio.

**Protocol**: JSON lines (`\n`-delimited JSON objects) over the pipe — simple to debug, implement, and replace later with a typed protocol.

### Phase 2: Typed Protocol
Once workflows stabilize, replace JSON with the `iagent-ipc-types` serde-serializable types crate.

---

## What's NOT Ready for Boundary Separation

1. **`desktop_ambient`** directly calls `init_provider_and_registry()` which spawns the full backend provider layer — this initialization would need to move to the backend side
2. **Config access** — `config().ambient.*` reads in-process TOML/JSON config; config loading would need to move to backend side with IPC query back
3. **Provider initialization** — the `Provider::new()` construction happens in the same process as the overlay

---

## New IPC Crate: `iagent-ipc-types`

Should define only:
- `serde`-serializable request/response types
- No async, no tokio, no heavy deps
- uuid for session IDs

The `LanguageModelProvider` trait is NOT a good candidate for the IPC types crate — it crosses the boundary as a serializable prompt text + response text, not as a typed fn pointer.

---

## Next Actions

1. ✅ Document this audit
2. ⬜ Create `crates/iagent-ipc-types` crate with request/response types
3. ⬜ Sketch named pipe transport stub
4. ⬜ Refactor `desktop_ambient` to split shell process vs backend process
