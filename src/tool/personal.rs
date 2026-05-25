use super::{Tool, ToolContext, ToolOutput};
use crate::personal_layer::{
    ClearPersonalData, ClipboardInput, ComputerUseRequest, JobInput, PersonalSettingsInput,
    PersonalStore, ProjectWorkspaceInput, ReminderInput, RuntimeTickInput, SavedWindowLayoutInput,
    SnippetInput, TimelineEntryInput, TimelineSearch, WindowBounds, WindowPlacement,
    plan_snap_window, plan_tile_two_windows, snap_active_window,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct PersonalTool;

impl PersonalTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct PersonalInput {
    action: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    trigger: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    app_scope: Vec<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    due_at: Option<String>,
    #[serde(default)]
    snooze_until: Option<String>,
    #[serde(default)]
    as_of: Option<String>,
    #[serde(default)]
    source_app: Option<String>,
    #[serde(default)]
    source_title: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    process_name: Option<String>,
    #[serde(default)]
    exe_path: Option<String>,
    #[serde(default)]
    window_title: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    left_query: Option<String>,
    #[serde(default)]
    right_query: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    input_json: Option<Value>,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default)]
    monitor: Option<WindowBounds>,
    #[serde(default)]
    placements: Vec<WindowPlacement>,
    #[serde(default)]
    clipboard_history_enabled: Option<bool>,
    #[serde(default)]
    reminder_notifications_enabled: Option<bool>,
    #[serde(default)]
    background_jobs_enabled: Option<bool>,
    #[serde(default)]
    proactive_suggestions_enabled: Option<bool>,
    #[serde(default)]
    snippet_expansion_enabled: Option<bool>,
    #[serde(default)]
    max_clipboard_entries: Option<usize>,
    #[serde(default)]
    retention_days: Option<u32>,
    #[serde(default)]
    pinned: Option<bool>,
    #[serde(default)]
    run_one_job: Option<bool>,
    #[serde(default)]
    active_app: Option<String>,
    #[serde(default)]
    active_window_title: Option<String>,
    #[serde(default)]
    clear_clipboard: bool,
    #[serde(default)]
    clear_reminders: bool,
    #[serde(default)]
    clear_snippets: bool,
    #[serde(default)]
    clear_jobs: bool,
    #[serde(default)]
    clear_app_windows: bool,
    #[serde(default)]
    clear_layouts: bool,
    #[serde(default)]
    clear_timeline: bool,
    #[serde(default)]
    clear_workspaces: bool,
    #[serde(default)]
    timeline_enabled: Option<bool>,
    #[serde(default)]
    app_history_enabled: Option<bool>,
    #[serde(default)]
    screenshots_enabled: Option<bool>,
    #[serde(default)]
    ocr_enabled: Option<bool>,
    #[serde(default)]
    uia_text_enabled: Option<bool>,
    #[serde(default)]
    computer_use_enabled: Option<bool>,
    #[serde(default)]
    prompt_injection_defense_enabled: Option<bool>,
    #[serde(default)]
    encrypted_sensitive_storage: Option<bool>,
    #[serde(default)]
    require_approval_for_personal_actions: Option<bool>,
    #[serde(default)]
    excluded_apps: Option<Vec<String>>,
    #[serde(default)]
    private_title_patterns: Option<Vec<String>>,
    #[serde(default)]
    layout_name: Option<String>,
    #[serde(default)]
    app_queries: Vec<String>,
    #[serde(default)]
    observation_text: Option<String>,
}

#[async_trait]
impl Tool for PersonalTool {
    fn name(&self) -> &str {
        "personal"
    }

    fn description(&self) -> &str {
        "Manage iAgent's personal desktop layer: snippets, contextual reminders, clipboard recovery, recent app/window recall, background job records, and window layout plans."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": [
                        "get_settings", "update_settings", "runtime_tick", "clear_personal_data",
                        "control_panel",
                        "create_snippet", "list_snippets", "expand_snippet", "delete_snippet",
                        "expand_typed_snippet",
                        "create_reminder", "list_reminders", "due_reminders", "complete_reminder", "snooze_reminder",
                        "record_clipboard", "capture_clipboard", "clipboard_recent", "clipboard_clear",
                        "clipboard_pin", "clipboard_delete",
                        "record_timeline", "search_timeline", "delete_timeline",
                        "computer_use_plan",
                        "record_app_window", "capture_active_window", "list_recent_apps", "resolve_app", "switch_to_app",
                        "create_job", "list_jobs", "cancel_job", "retry_job", "run_job", "run_next_job",
                        "snap_window_plan", "tile_windows_plan", "save_window_layout", "list_window_layouts", "window_layout_plan",
                        "save_project_workspace", "list_project_workspaces",
                        "snap_active_window", "tile_windows"
                    ]
                },
                "id": {"type": "string"},
                "trigger": {"type": "string"},
                "body": {"type": "string"},
                "description": {"type": "string"},
                "app_scope": {"type": "array", "items": {"type": "string"}},
                "title": {"type": "string"},
                "note": {"type": "string"},
                "due_at": {"type": "string", "description": "RFC3339 due time."},
                "snooze_until": {"type": "string", "description": "RFC3339 snooze time."},
                "as_of": {"type": "string", "description": "RFC3339 time for due reminder checks."},
                "source_app": {"type": "string"},
                "source_title": {"type": "string"},
                "content": {"type": "string"},
                "limit": {"type": "integer"},
                "process_name": {"type": "string"},
                "exe_path": {"type": "string"},
                "window_title": {"type": "string"},
                "query": {"type": "string"},
                "left_query": {"type": "string"},
                "right_query": {"type": "string"},
                "kind": {"type": "string"},
                "input_json": {"type": "object", "additionalProperties": true},
                "placements": {"type": "array", "items": {"type": "object"}},
                "clipboard_history_enabled": {"type": "boolean"},
                "reminder_notifications_enabled": {"type": "boolean"},
                "background_jobs_enabled": {"type": "boolean"},
                "proactive_suggestions_enabled": {"type": "boolean"},
                "snippet_expansion_enabled": {"type": "boolean"},
                "max_clipboard_entries": {"type": "integer"},
                "retention_days": {"type": "integer"},
                "pinned": {"type": "boolean"},
                "run_one_job": {"type": "boolean"},
                "active_app": {"type": "string"},
                "active_window_title": {"type": "string"},
                "clear_clipboard": {"type": "boolean"},
                "clear_reminders": {"type": "boolean"},
                "clear_snippets": {"type": "boolean"},
                "clear_jobs": {"type": "boolean"},
                "clear_app_windows": {"type": "boolean"},
                "clear_layouts": {"type": "boolean"},
                "clear_timeline": {"type": "boolean"},
                "clear_workspaces": {"type": "boolean"},
                "timeline_enabled": {"type": "boolean"},
                "app_history_enabled": {"type": "boolean"},
                "screenshots_enabled": {"type": "boolean"},
                "ocr_enabled": {"type": "boolean"},
                "uia_text_enabled": {"type": "boolean"},
                "computer_use_enabled": {"type": "boolean"},
                "prompt_injection_defense_enabled": {"type": "boolean"},
                "encrypted_sensitive_storage": {"type": "boolean"},
                "require_approval_for_personal_actions": {"type": "boolean"},
                "excluded_apps": {"type": "array", "items": {"type": "string"}},
                "private_title_patterns": {"type": "array", "items": {"type": "string"}},
                "layout_name": {"type": "string"},
                "app_queries": {"type": "array", "items": {"type": "string"}},
                "observation_text": {"type": "string"},
                "direction": {"type": "string", "enum": ["left", "right", "top", "bottom", "center", "maximize", "full"]},
                "monitor": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"},
                        "y": {"type": "integer"},
                        "width": {"type": "integer"},
                        "height": {"type": "integer"}
                    },
                    "required": ["x", "y", "width", "height"]
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: PersonalInput = serde_json::from_value(input)?;
        let store = PersonalStore::load()?;

        match input.action.as_str() {
            "get_settings" => Ok(ToolOutput::new(serde_json::to_string_pretty(
                &store.settings()?,
            )?)),
            "update_settings" => {
                let settings = store.update_settings(PersonalSettingsInput {
                    clipboard_history_enabled: input.clipboard_history_enabled,
                    reminder_notifications_enabled: input.reminder_notifications_enabled,
                    background_jobs_enabled: input.background_jobs_enabled,
                    proactive_suggestions_enabled: input.proactive_suggestions_enabled,
                    snippet_expansion_enabled: input.snippet_expansion_enabled,
                    max_clipboard_entries: input.max_clipboard_entries,
                    retention_days: input.retention_days,
                    timeline_enabled: input.timeline_enabled,
                    app_history_enabled: input.app_history_enabled,
                    screenshots_enabled: input.screenshots_enabled,
                    ocr_enabled: input.ocr_enabled,
                    uia_text_enabled: input.uia_text_enabled,
                    computer_use_enabled: input.computer_use_enabled,
                    prompt_injection_defense_enabled: input.prompt_injection_defense_enabled,
                    encrypted_sensitive_storage: input.encrypted_sensitive_storage,
                    require_approval_for_personal_actions: input
                        .require_approval_for_personal_actions,
                    excluded_apps: input.excluded_apps,
                    private_title_patterns: input.private_title_patterns,
                })?;
                Ok(ToolOutput::new(format!(
                    "Personal settings updated: clipboard_history={} reminders={} jobs={} proactive={} snippets={} timeline={} computer_use={} max_clipboard_entries={} retention_days={}",
                    settings.clipboard_history_enabled,
                    settings.reminder_notifications_enabled,
                    settings.background_jobs_enabled,
                    settings.proactive_suggestions_enabled,
                    settings.snippet_expansion_enabled,
                    settings.timeline_enabled,
                    settings.computer_use_enabled,
                    settings.max_clipboard_entries,
                    settings.retention_days
                )))
            }
            "runtime_tick" => {
                let tick = store.run_runtime_tick(RuntimeTickInput {
                    as_of: input
                        .as_of
                        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                    clipboard_content: input.content,
                    active_app: input.active_app.or(input.source_app),
                    active_window_title: input.active_window_title.or(input.source_title),
                    run_one_job: input.run_one_job.unwrap_or(true),
                })?;
                Ok(ToolOutput::new(format!(
                    "Runtime tick: {} due reminder(s), clipboard_captured={}, job_completed={}, {} suggestion(s)",
                    tick.due_reminders.len(),
                    tick.captured_clipboard.is_some(),
                    tick.completed_job
                        .as_ref()
                        .map(|job| job.status.as_str())
                        .unwrap_or("none"),
                    tick.suggestions.len()
                )))
            }
            "clear_personal_data" => {
                let cleared = store.clear_personal_data(ClearPersonalData {
                    clipboard: input.clear_clipboard,
                    reminders: input.clear_reminders,
                    snippets: input.clear_snippets,
                    jobs: input.clear_jobs,
                    app_windows: input.clear_app_windows,
                    layouts: input.clear_layouts,
                    timeline: input.clear_timeline,
                    workspaces: input.clear_workspaces,
                })?;
                Ok(ToolOutput::new(format!(
                    "Cleared personal data: clipboard={} reminders={} snippets={} jobs={} app_windows={} layouts={} timeline={} workspaces={}",
                    cleared.clipboard,
                    cleared.reminders,
                    cleared.snippets,
                    cleared.jobs,
                    cleared.app_windows,
                    cleared.layouts,
                    cleared.timeline,
                    cleared.workspaces
                )))
            }
            "control_panel" => Ok(ToolOutput::new(serde_json::to_string_pretty(
                &store.control_panel_summary()?,
            )?)),
            "create_snippet" => {
                let snippet = store.create_snippet(SnippetInput {
                    trigger: required(input.trigger, "trigger")?,
                    body: required(input.body, "body")?,
                    description: input.description,
                    app_scope: input.app_scope,
                })?;
                Ok(ToolOutput::new(format!(
                    "Snippet saved: {} [id: {}]",
                    snippet.trigger, snippet.id
                )))
            }
            "list_snippets" => {
                let snippets = store.list_snippets()?;
                Ok(ToolOutput::new(if snippets.is_empty() {
                    "No snippets saved.".to_string()
                } else {
                    snippets
                        .into_iter()
                        .map(|snippet| format!("- {} [id: {}]", snippet.trigger, snippet.id))
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "expand_snippet" => {
                let trigger = required(input.trigger, "trigger")?;
                Ok(ToolOutput::new(store.expand_snippet(&trigger)?))
            }
            "expand_typed_snippet" => {
                let text = required(input.content, "content")?;
                Ok(ToolOutput::new(
                    store
                        .expand_typed_snippet(&text, input.source_app.as_deref())?
                        .map(|expansion| expansion.output_text)
                        .unwrap_or(text),
                ))
            }
            "delete_snippet" => {
                let id = input
                    .id
                    .or(input.trigger)
                    .ok_or_else(|| anyhow!("id or trigger required"))?;
                Ok(ToolOutput::new(if store.delete_snippet(&id)? {
                    format!("Deleted snippet: {}", id)
                } else {
                    format!("Snippet not found: {}", id)
                }))
            }
            "create_reminder" => {
                let reminder = store.create_reminder(ReminderInput {
                    title: required(input.title, "title")?,
                    note: input.note,
                    due_at: required(input.due_at, "due_at")?,
                    source_app: input.source_app,
                    source_title: input.source_title,
                })?;
                Ok(ToolOutput::new(format!(
                    "Reminder saved: {} at {} [id: {}]",
                    reminder.title, reminder.due_at, reminder.id
                )))
            }
            "list_reminders" => {
                let reminders = store.list_pending_reminders()?;
                Ok(ToolOutput::new(if reminders.is_empty() {
                    "No pending reminders.".to_string()
                } else {
                    reminders
                        .into_iter()
                        .map(|reminder| {
                            format!(
                                "- {} at {} [id: {}]",
                                reminder.title, reminder.due_at, reminder.id
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "due_reminders" => {
                let as_of = input
                    .as_of
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                let reminders = store.list_due_reminders(&as_of)?;
                Ok(ToolOutput::new(if reminders.is_empty() {
                    "No due reminders.".to_string()
                } else {
                    reminders
                        .into_iter()
                        .map(|reminder| {
                            let context = match (&reminder.source_app, &reminder.source_title) {
                                (Some(app), Some(title)) => format!(" [{} - {}]", app, title),
                                (Some(app), None) => format!(" [{}]", app),
                                (None, Some(title)) => format!(" [{}]", title),
                                (None, None) => String::new(),
                            };
                            format!(
                                "- {} at {}{} [id: {}]",
                                reminder.title, reminder.due_at, context, reminder.id
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "complete_reminder" => {
                let id = required(input.id, "id")?;
                Ok(ToolOutput::new(if store.complete_reminder(&id)? {
                    format!("Completed reminder: {}", id)
                } else {
                    format!("Reminder not found: {}", id)
                }))
            }
            "snooze_reminder" => {
                let id = required(input.id, "id")?;
                let until = required(input.snooze_until, "snooze_until")?;
                Ok(ToolOutput::new(if store.snooze_reminder(&id, &until)? {
                    format!("Snoozed reminder: {}", id)
                } else {
                    format!("Reminder not found: {}", id)
                }))
            }
            "record_clipboard" => {
                let entry = store.record_clipboard(ClipboardInput {
                    content: required(input.content, "content")?,
                    source_app: input.source_app,
                })?;
                Ok(ToolOutput::new(match entry {
                    Some(entry) if entry.redacted => {
                        format!("Clipboard entry skipped/redacted [id: {}]", entry.id)
                    }
                    Some(entry) => format!("Clipboard entry recorded [id: {}]", entry.id),
                    None => "Clipboard entry ignored.".to_string(),
                }))
            }
            "capture_clipboard" => {
                let entry = store.capture_system_clipboard(input.source_app)?;
                Ok(ToolOutput::new(match entry {
                    Some(entry) if entry.redacted => {
                        format!("Clipboard text captured but redacted [id: {}]", entry.id)
                    }
                    Some(entry) => format!("Clipboard text captured [id: {}]", entry.id),
                    None => "Clipboard text ignored.".to_string(),
                }))
            }
            "clipboard_recent" => {
                let entries = store.recent_clipboard(input.limit.unwrap_or(10))?;
                Ok(ToolOutput::new(if entries.is_empty() {
                    "No clipboard history.".to_string()
                } else {
                    entries
                        .into_iter()
                        .map(|entry| {
                            let text = entry
                                .content_text
                                .unwrap_or_else(|| "<redacted>".to_string());
                            format!("- {} [id: {}]", text, entry.id)
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "clipboard_clear" => {
                let count = store.clear_clipboard()?;
                Ok(ToolOutput::new(format!(
                    "Cleared {} clipboard entries.",
                    count
                )))
            }
            "clipboard_pin" => {
                let id = required(input.id, "id")?;
                let pinned = input.pinned.unwrap_or(true);
                Ok(ToolOutput::new(if store.pin_clipboard(&id, pinned)? {
                    format!("Clipboard entry updated: {} pinned={}", id, pinned)
                } else {
                    format!("Clipboard entry not found: {}", id)
                }))
            }
            "clipboard_delete" => {
                let id = required(input.id, "id")?;
                Ok(ToolOutput::new(if store.delete_clipboard(&id)? {
                    format!("Deleted clipboard entry: {}", id)
                } else {
                    format!("Clipboard entry not found: {}", id)
                }))
            }
            "record_timeline" => {
                let entry = store.record_timeline_entry(TimelineEntryInput {
                    app_name: required(input.source_app.or(input.active_app), "source_app")?,
                    window_title: required(
                        input.source_title.or(input.active_window_title),
                        "source_title",
                    )?,
                    activity: input
                        .description
                        .unwrap_or_else(|| "Observed activity".to_string()),
                    text_excerpt: input.content,
                    screenshot_path: input
                        .input_json
                        .as_ref()
                        .and_then(|value| value.get("screenshot_path"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    source: input.kind.unwrap_or_else(|| "manual".to_string()),
                })?;
                Ok(ToolOutput::new(
                    entry
                        .map(|entry| format!("Timeline entry recorded [id: {}]", entry.id))
                        .unwrap_or_else(|| {
                            "Timeline entry skipped by privacy settings.".to_string()
                        }),
                ))
            }
            "search_timeline" => {
                let entries = store.search_timeline(TimelineSearch {
                    query: input.query,
                    app_name: input.source_app.or(input.active_app),
                    limit: input.limit.unwrap_or(10),
                })?;
                Ok(ToolOutput::new(if entries.is_empty() {
                    "No timeline entries matched.".to_string()
                } else {
                    entries
                        .into_iter()
                        .map(|entry| {
                            format!(
                                "- {} - {}: {} [id: {}]",
                                entry.app_name, entry.window_title, entry.activity, entry.id
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "delete_timeline" => {
                let id = required(input.id, "id")?;
                Ok(ToolOutput::new(if store.delete_timeline_entry(&id)? {
                    format!("Deleted timeline entry: {}", id)
                } else {
                    format!("Timeline entry not found: {}", id)
                }))
            }
            "computer_use_plan" => {
                let plan = store.draft_computer_use_plan(ComputerUseRequest {
                    goal: required(input.description.or(input.query), "description")?,
                    app_name: input.source_app.or(input.active_app),
                    window_title: input.source_title.or(input.active_window_title),
                    observation_text: input.observation_text.or(input.content),
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&plan)?))
            }
            "record_app_window" => {
                let record = store.record_app_window(
                    &required(input.process_name, "process_name")?,
                    &input.exe_path.unwrap_or_default(),
                    &required(input.window_title, "window_title")?,
                )?;
                Ok(ToolOutput::new(format!(
                    "Recorded app/window: {} - {} [id: {}]",
                    record.process_name, record.window_title, record.id
                )))
            }
            "capture_active_window" => Ok(ToolOutput::new(
                store
                    .capture_active_window()?
                    .map(|record| {
                        format!(
                            "Captured active window: {} - {} [id: {}]",
                            record.process_name, record.window_title, record.id
                        )
                    })
                    .unwrap_or_else(|| "No active window context available.".to_string()),
            )),
            "list_recent_apps" => {
                let records = store.list_recent_app_windows(input.limit.unwrap_or(10))?;
                Ok(ToolOutput::new(if records.is_empty() {
                    "No recent app/window history.".to_string()
                } else {
                    records
                        .into_iter()
                        .map(|record| {
                            format!(
                                "- {} - {} [id: {}]",
                                record.process_name, record.window_title, record.id
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "resolve_app" => {
                let query = required(input.query, "query")?;
                Ok(ToolOutput::new(
                    store
                        .resolve_app_description(&query)?
                        .map(|record| {
                            format!(
                                "Resolved to {} - {} [id: {}]",
                                record.process_name, record.window_title, record.id
                            )
                        })
                        .unwrap_or_else(|| format!("No app/window matched '{}'.", query)),
                ))
            }
            "switch_to_app" => {
                let query = required(input.query, "query")?;
                Ok(ToolOutput::new(
                    store
                        .switch_to_app_description(&query)?
                        .map(|record| {
                            format!(
                                "Switched to {} - {} [id: {}]",
                                record.process_name, record.window_title, record.id
                            )
                        })
                        .unwrap_or_else(|| format!("No app/window matched '{}'.", query)),
                ))
            }
            "create_job" => {
                let job = store.create_job(JobInput {
                    kind: required(input.kind, "kind")?,
                    description: input.description.unwrap_or_default(),
                    input_json: input.input_json.unwrap_or_else(|| json!({})),
                })?;
                Ok(ToolOutput::new(format!(
                    "Job queued: {} [id: {}]",
                    job.kind, job.id
                )))
            }
            "list_jobs" => {
                let jobs = store.list_jobs()?;
                Ok(ToolOutput::new(if jobs.is_empty() {
                    "No jobs queued.".to_string()
                } else {
                    jobs.into_iter()
                        .map(|job| format!("- {} {} [id: {}]", job.status, job.kind, job.id))
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "cancel_job" => {
                let id = required(input.id, "id")?;
                Ok(ToolOutput::new(if store.cancel_job(&id)? {
                    format!("Cancelled job: {}", id)
                } else {
                    format!("Job not cancellable or not found: {}", id)
                }))
            }
            "retry_job" => {
                let id = required(input.id, "id")?;
                Ok(ToolOutput::new(
                    store
                        .retry_job(&id)?
                        .map(|job| format!("Retried job: {} [id: {}]", job.kind, job.id))
                        .unwrap_or_else(|| format!("Job not found: {}", id)),
                ))
            }
            "run_job" => {
                let id = required(input.id, "id")?;
                Ok(ToolOutput::new(
                    store
                        .run_job(&id)?
                        .map(|job| format!("Job {}: {} [id: {}]", job.status, job.kind, job.id))
                        .unwrap_or_else(|| format!("Job not found: {}", id)),
                ))
            }
            "run_next_job" => Ok(ToolOutput::new(
                store
                    .run_next_job()?
                    .map(|job| format!("Job {}: {} [id: {}]", job.status, job.kind, job.id))
                    .unwrap_or_else(|| "No pending jobs.".to_string()),
            )),
            "snap_window_plan" => {
                let monitor = input
                    .monitor
                    .ok_or_else(|| anyhow!("monitor bounds required"))?;
                let direction = required(input.direction, "direction")?;
                let bounds = plan_snap_window(monitor, &direction)?;
                Ok(ToolOutput::new(format!(
                    "Window plan: x={} y={} width={} height={}",
                    bounds.x, bounds.y, bounds.width, bounds.height
                )))
            }
            "tile_windows_plan" => {
                let monitor = input
                    .monitor
                    .ok_or_else(|| anyhow!("monitor bounds required"))?;
                let placements = plan_tile_two_windows(monitor)?;
                Ok(ToolOutput::new(format_placements(&placements)))
            }
            "save_window_layout" => {
                let layout = store.save_window_layout(SavedWindowLayoutInput {
                    name: required(input.title.or(input.description), "title")?,
                    placements: input.placements,
                })?;
                Ok(ToolOutput::new(format!(
                    "Saved window layout: {} [id: {}]",
                    layout.name, layout.id
                )))
            }
            "list_window_layouts" => {
                let layouts = store.list_window_layouts()?;
                Ok(ToolOutput::new(if layouts.is_empty() {
                    "No saved window layouts.".to_string()
                } else {
                    layouts
                        .into_iter()
                        .map(|layout| {
                            format!(
                                "- {} ({} placement(s)) [id: {}]",
                                layout.name,
                                layout.placements.len(),
                                layout.id
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "window_layout_plan" => {
                let name = required(input.title.or(input.id), "title")?;
                Ok(ToolOutput::new(
                    store
                        .saved_window_layout_plan(&name)?
                        .map(|placements| format_placements(&placements))
                        .unwrap_or_else(|| format!("Window layout not found: {}", name)),
                ))
            }
            "save_project_workspace" => {
                let workspace = store.save_project_workspace(ProjectWorkspaceInput {
                    name: required(input.title.or(input.description), "title")?,
                    layout_name: input.layout_name,
                    app_queries: input.app_queries,
                    notes: input.note,
                })?;
                Ok(ToolOutput::new(format!(
                    "Saved project workspace: {} [id: {}]",
                    workspace.name, workspace.id
                )))
            }
            "list_project_workspaces" => {
                let workspaces = store.list_project_workspaces()?;
                Ok(ToolOutput::new(if workspaces.is_empty() {
                    "No project workspaces saved.".to_string()
                } else {
                    workspaces
                        .into_iter()
                        .map(|workspace| {
                            format!(
                                "- {} ({} app query/queries) [id: {}]",
                                workspace.name,
                                workspace.app_queries.len(),
                                workspace.id
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }))
            }
            "snap_active_window" => {
                let direction = required(input.direction, "direction")?;
                let bounds = snap_active_window(&direction)?;
                Ok(ToolOutput::new(format!(
                    "Moved active window: x={} y={} width={} height={}",
                    bounds.x, bounds.y, bounds.width, bounds.height
                )))
            }
            "tile_windows" => {
                let left_query = required(input.left_query, "left_query")?;
                let right_query = required(input.right_query, "right_query")?;
                Ok(ToolOutput::new(
                    store
                        .tile_app_descriptions(&left_query, &right_query)?
                        .map(|placements| {
                            format!("Tiled windows:\n{}", format_placements(&placements))
                        })
                        .unwrap_or_else(|| {
                            format!(
                                "Could not resolve both windows for '{}' and '{}'.",
                                left_query, right_query
                            )
                        }),
                ))
            }
            other => Err(anyhow!("Unknown personal action: {}", other)),
        }
    }
}

fn format_placements(placements: &[WindowPlacement]) -> String {
    placements
        .iter()
        .map(|placement| {
            format!(
                "- {}: x={} y={} width={} height={}",
                placement.label,
                placement.bounds.x,
                placement.bounds.y,
                placement.bounds.width,
                placement.bounds.height
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{} required", name))
}
