# Personal Product Layer Implementation Plan

> For agentic workers: implement in scoped slices with tests first. Preserve unrelated ambient/memory worktree changes.

**Goal:** Turn the manual personal-layer tools into a shippable local-first product surface with always-on watchers, searchable timeline, safer computer-use planning, proactive suggestions, privacy controls, and UI-ready summaries.

**Architecture:** Keep durable state in `PersonalStore`, run continuous observation through `personal_daemon`, and expose agent/UI controls through the `personal` tool and CLI. Native hooks feed snapshots into the same tested APIs instead of writing directly to storage.

**Implementation Slices**

- [x] Personal daemon contract: periodic tick, clipboard/window snapshot, reminder notifications, one-job worker pass, status summary.
- [x] Reminder scheduler contract: due reminders become notification events from daemon ticks.
- [x] Job daemon contract: queued built-in jobs run from daemon ticks and persist JSON logs.
- [x] Typed snippet core: app-scoped expansion is available for a global keyboard hook to call.
- [x] Native global keyboard hook: the Windows personal daemon feeds typed triggers into `expand_typed_snippet`.
- [x] Recall-like timeline core: privacy-aware timeline entries, search, retention cap, app/title exclusions, delete controls.
- [x] Computer-use loop contract: observation/action/verification plan with prompt-injection risk flags and permission tier.
- [x] Proactive suggestions: runtime ticks emit remember/snippet/reminder/job-style suggestion events.
- [x] UI control panels: backend summary for memory, clipboard, reminders, snippets, jobs, privacy, layouts, timeline, and workspaces.
- [x] Settings surface: the dock Settings > Personal panel can refresh daemon status, run one tick, start the daemon, and open the personal-data folder.
- [x] Safety/privacy controls: retention settings, app exclusions, private-title patterns, capture-mode toggles, prompt-injection defense flag.
- [x] Saved layouts/workspaces: named layouts plus project workspace records for richer restore and disambiguation UI.
- [ ] Native screenshots/OCR/UIA capture: plug capture providers into `record_timeline_entry`.
- [ ] Polished GUI panels: render detailed list actions for snippets, reminders, clipboard, jobs, privacy, layouts, timeline, and workspaces in the dock/tray UI.
