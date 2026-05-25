use super::{Tool, ToolContext, ToolOutput};
use crate::personal_layer::{
    ClipboardInput, JobInput, PersonalStore, ReminderInput, SnippetInput, WindowBounds,
    WindowPlacement, plan_snap_window, plan_tile_two_windows, snap_active_window,
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
                        "create_snippet", "list_snippets", "expand_snippet", "delete_snippet",
                        "create_reminder", "list_reminders", "due_reminders", "complete_reminder", "snooze_reminder",
                        "record_clipboard", "capture_clipboard", "clipboard_recent", "clipboard_clear",
                        "record_app_window", "capture_active_window", "list_recent_apps", "resolve_app", "switch_to_app",
                        "create_job", "list_jobs", "cancel_job", "retry_job", "run_job", "run_next_job",
                        "snap_window_plan", "tile_windows_plan", "snap_active_window", "tile_windows"
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
