use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use uuid::Uuid;

const MAX_CLIPBOARD_ENTRIES: usize = 25;

#[derive(Debug, Clone)]
pub struct PersonalStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonalState {
    #[serde(default)]
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub reminders: Vec<Reminder>,
    #[serde(default)]
    pub clipboard: Vec<ClipboardEntry>,
    #[serde(default)]
    pub app_windows: Vec<AppWindowRecord>,
    #[serde(default)]
    pub jobs: Vec<BackgroundJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snippet {
    pub id: String,
    pub trigger: String,
    pub body: String,
    pub description: Option<String>,
    #[serde(default)]
    pub app_scope: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SnippetInput {
    pub trigger: String,
    pub body: String,
    pub description: Option<String>,
    pub app_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub snoozed_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ReminderInput {
    pub title: String,
    pub note: Option<String>,
    pub due_at: String,
    pub source_app: Option<String>,
    pub source_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardEntry {
    pub id: String,
    pub captured_at: DateTime<Utc>,
    pub content_hash: String,
    pub content_text: Option<String>,
    pub source_app: Option<String>,
    pub pinned: bool,
    pub redacted: bool,
}

#[derive(Debug, Clone)]
pub struct ClipboardInput {
    pub content: String,
    pub source_app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppWindowRecord {
    pub id: String,
    pub observed_at: DateTime<Utc>,
    pub process_name: String,
    pub exe_path: String,
    pub window_title: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackgroundJob {
    pub id: String,
    pub kind: String,
    pub description: String,
    pub status: String,
    pub input_json: Value,
    pub output_json: Option<Value>,
    pub log_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub cancel_requested_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct JobInput {
    pub kind: String,
    pub description: String,
    pub input_json: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl PersonalStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::jcode_dir()?.join("personal");
        fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("personal_layer.json"),
        })
    }

    pub fn state(&self) -> Result<PersonalState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read personal layer state at {}", self.path.display()))
        } else {
            Ok(PersonalState::default())
        }
    }

    fn save_state(&self, state: &PersonalState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write personal layer state at {}", self.path.display()))
    }

    pub fn create_snippet(&self, input: SnippetInput) -> Result<Snippet> {
        validate_trigger(&input.trigger)?;
        if input.body.trim().is_empty() {
            return Err(anyhow!("snippet body is required"));
        }

        let mut state = self.state()?;
        let now = Utc::now();
        if let Some(existing) = state
            .snippets
            .iter_mut()
            .find(|snippet| snippet.trigger == input.trigger)
        {
            existing.body = input.body;
            existing.description = input.description;
            existing.app_scope = input.app_scope;
            existing.enabled = true;
            existing.updated_at = now;
            let snippet = existing.clone();
            self.save_state(&state)?;
            return Ok(snippet);
        }

        let snippet = Snippet {
            id: Uuid::new_v4().to_string(),
            trigger: input.trigger,
            body: input.body,
            description: input.description,
            app_scope: input.app_scope,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        state.snippets.push(snippet.clone());
        self.save_state(&state)?;
        Ok(snippet)
    }

    pub fn list_snippets(&self) -> Result<Vec<Snippet>> {
        let mut snippets = self.state()?.snippets;
        snippets.sort_by(|a, b| a.trigger.cmp(&b.trigger));
        Ok(snippets)
    }

    pub fn expand_snippet(&self, trigger: &str) -> Result<String> {
        let state = self.state()?;
        state
            .snippets
            .into_iter()
            .find(|snippet| snippet.enabled && snippet.trigger == trigger)
            .map(|snippet| snippet.body)
            .ok_or_else(|| anyhow!("snippet not found: {}", trigger))
    }

    pub fn delete_snippet(&self, id_or_trigger: &str) -> Result<bool> {
        let mut state = self.state()?;
        let before = state.snippets.len();
        state
            .snippets
            .retain(|snippet| snippet.id != id_or_trigger && snippet.trigger != id_or_trigger);
        let removed = state.snippets.len() != before;
        if removed {
            self.save_state(&state)?;
        }
        Ok(removed)
    }

    pub fn create_reminder(&self, input: ReminderInput) -> Result<Reminder> {
        if input.title.trim().is_empty() {
            return Err(anyhow!("reminder title is required"));
        }
        let due_at = parse_rfc3339_utc(&input.due_at)?;
        let now = Utc::now();
        let reminder = Reminder {
            id: Uuid::new_v4().to_string(),
            title: input.title,
            note: input.note,
            due_at,
            status: "pending".to_string(),
            source_app: input.source_app,
            source_title: input.source_title,
            created_at: now,
            completed_at: None,
            snoozed_until: None,
        };
        let mut state = self.state()?;
        state.reminders.push(reminder.clone());
        self.save_state(&state)?;
        Ok(reminder)
    }

    pub fn list_pending_reminders(&self) -> Result<Vec<Reminder>> {
        let mut reminders: Vec<_> = self
            .state()?
            .reminders
            .into_iter()
            .filter(|reminder| reminder.status == "pending")
            .collect();
        reminders.sort_by(|a, b| a.due_at.cmp(&b.due_at));
        Ok(reminders)
    }

    pub fn complete_reminder(&self, id: &str) -> Result<bool> {
        self.update_reminder_status(id, "completed", Some(Utc::now()), None)
    }

    pub fn snooze_reminder(&self, id: &str, until: &str) -> Result<bool> {
        let until = parse_rfc3339_utc(until)?;
        self.update_reminder_status(id, "pending", None, Some(until))
    }

    fn update_reminder_status(
        &self,
        id: &str,
        status: &str,
        completed_at: Option<DateTime<Utc>>,
        snoozed_until: Option<DateTime<Utc>>,
    ) -> Result<bool> {
        let mut state = self.state()?;
        let mut found = false;
        for reminder in &mut state.reminders {
            if reminder.id == id {
                reminder.status = status.to_string();
                reminder.completed_at = completed_at;
                reminder.snoozed_until = snoozed_until;
                found = true;
            }
        }
        if found {
            self.save_state(&state)?;
        }
        Ok(found)
    }

    pub fn record_clipboard(&self, input: ClipboardInput) -> Result<Option<ClipboardEntry>> {
        if input.content.trim().is_empty() {
            return Ok(None);
        }
        let hash = stable_hash(&input.content);
        let mut state = self.state()?;
        if state.clipboard.iter().any(|entry| {
            entry.content_hash == hash && entry.content_text.as_deref() == Some(&input.content)
        }) {
            return Ok(None);
        }

        let redacted = looks_secret(&input.content);
        let entry = ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            captured_at: Utc::now(),
            content_hash: hash,
            content_text: if redacted { None } else { Some(input.content) },
            source_app: input.source_app,
            pinned: false,
            redacted,
        };
        state.clipboard.insert(0, entry.clone());
        state.clipboard.truncate(MAX_CLIPBOARD_ENTRIES);
        self.save_state(&state)?;
        Ok(Some(entry))
    }

    pub fn capture_system_clipboard(
        &self,
        source_app: Option<String>,
    ) -> Result<Option<ClipboardEntry>> {
        let mut clipboard = arboard::Clipboard::new().context("open system clipboard")?;
        let content = clipboard
            .get_text()
            .context("read text from system clipboard")?;
        self.record_clipboard(ClipboardInput {
            content,
            source_app,
        })
    }

    pub fn recent_clipboard(&self, limit: usize) -> Result<Vec<ClipboardEntry>> {
        Ok(self
            .state()?
            .clipboard
            .into_iter()
            .take(limit.max(1))
            .collect())
    }

    pub fn clear_clipboard(&self) -> Result<usize> {
        let mut state = self.state()?;
        let count = state.clipboard.len();
        state.clipboard.clear();
        self.save_state(&state)?;
        Ok(count)
    }

    pub fn record_app_window(
        &self,
        process_name: &str,
        exe_path: &str,
        window_title: &str,
    ) -> Result<AppWindowRecord> {
        if process_name.trim().is_empty() && window_title.trim().is_empty() {
            return Err(anyhow!("process_name or window_title is required"));
        }
        let record = AppWindowRecord {
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            process_name: process_name.to_string(),
            exe_path: exe_path.to_string(),
            window_title: window_title.to_string(),
            aliases: alias_terms(process_name, exe_path, window_title),
        };
        let mut state = self.state()?;
        state.app_windows.insert(0, record.clone());
        state.app_windows.truncate(100);
        self.save_state(&state)?;
        Ok(record)
    }

    pub fn list_recent_app_windows(&self, limit: usize) -> Result<Vec<AppWindowRecord>> {
        Ok(self
            .state()?
            .app_windows
            .into_iter()
            .take(limit.max(1))
            .collect())
    }

    pub fn resolve_app_description(&self, query: &str) -> Result<Option<AppWindowRecord>> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Ok(None);
        }
        Ok(self.state()?.app_windows.into_iter().find(|record| {
            let haystack = format!(
                "{} {} {} {}",
                record.process_name,
                record.exe_path,
                record.window_title,
                record.aliases.join(" ")
            )
            .to_lowercase();
            haystack.contains(&query)
                || query.split_whitespace().all(|term| haystack.contains(term))
        }))
    }

    pub fn create_job(&self, input: JobInput) -> Result<BackgroundJob> {
        if input.kind.trim().is_empty() {
            return Err(anyhow!("job kind is required"));
        }
        let job = BackgroundJob {
            id: Uuid::new_v4().to_string(),
            kind: input.kind,
            description: input.description,
            status: "pending".to_string(),
            input_json: input.input_json,
            output_json: None,
            log_path: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            cancel_requested_at: None,
        };
        let mut state = self.state()?;
        state.jobs.insert(0, job.clone());
        self.save_state(&state)?;
        Ok(job)
    }

    pub fn list_jobs(&self) -> Result<Vec<BackgroundJob>> {
        Ok(self.state()?.jobs)
    }

    pub fn cancel_job(&self, id: &str) -> Result<bool> {
        let mut state = self.state()?;
        let mut found = false;
        for job in &mut state.jobs {
            if job.id == id && matches!(job.status.as_str(), "pending" | "running") {
                job.status = "cancelled".to_string();
                job.cancel_requested_at = Some(Utc::now());
                job.finished_at = Some(Utc::now());
                found = true;
            }
        }
        if found {
            self.save_state(&state)?;
        }
        Ok(found)
    }

    pub fn retry_job(&self, id: &str) -> Result<Option<BackgroundJob>> {
        let state = self.state()?;
        let source = state.jobs.iter().find(|job| job.id == id).cloned();
        drop(state);
        if let Some(job) = source {
            return Ok(Some(self.create_job(JobInput {
                kind: job.kind,
                description: job.description,
                input_json: job.input_json,
            })?));
        }
        Ok(None)
    }

    pub fn run_next_job(&self) -> Result<Option<BackgroundJob>> {
        let id = self
            .state()?
            .jobs
            .iter()
            .rev()
            .find(|job| job.status == "pending")
            .map(|job| job.id.clone());

        match id {
            Some(id) => self.run_job(&id),
            None => Ok(None),
        }
    }

    pub fn run_job(&self, id: &str) -> Result<Option<BackgroundJob>> {
        let mut state = self.state()?;
        let Some(index) = state.jobs.iter().position(|job| job.id == id) else {
            return Ok(None);
        };

        if state.jobs[index].status != "pending" {
            return Ok(Some(state.jobs[index].clone()));
        }

        state.jobs[index].status = "running".to_string();
        state.jobs[index].started_at = Some(Utc::now());
        self.save_state(&state)?;

        let mut state = self.state()?;
        let job = state.jobs[index].clone();
        let result = run_builtin_job(&job);
        let finished_at = Utc::now();
        let updated = &mut state.jobs[index];
        updated.finished_at = Some(finished_at);
        match result {
            Ok(output) => {
                updated.status = "succeeded".to_string();
                updated.output_json = Some(output);
            }
            Err(err) => {
                updated.status = "failed".to_string();
                updated.output_json = Some(json!({ "error": err.to_string() }));
            }
        }
        let job = updated.clone();
        self.save_state(&state)?;
        Ok(Some(job))
    }
}

pub fn plan_snap_window(monitor: WindowBounds, direction: &str) -> Result<WindowBounds> {
    if monitor.width <= 0 || monitor.height <= 0 {
        return Err(anyhow!("monitor width and height must be positive"));
    }

    let half_width = monitor.width / 2;
    let half_height = monitor.height / 2;
    match direction {
        "left" => Ok(WindowBounds {
            width: half_width,
            ..monitor
        }),
        "right" => Ok(WindowBounds {
            x: monitor.x + half_width,
            width: monitor.width - half_width,
            ..monitor
        }),
        "top" => Ok(WindowBounds {
            height: half_height,
            ..monitor
        }),
        "bottom" => Ok(WindowBounds {
            y: monitor.y + half_height,
            height: monitor.height - half_height,
            ..monitor
        }),
        "center" => Ok(WindowBounds {
            x: monitor.x + monitor.width / 4,
            y: monitor.y + monitor.height / 4,
            width: half_width,
            height: half_height,
        }),
        "maximize" | "full" => Ok(monitor),
        other => Err(anyhow!(
            "unknown snap direction: {}. Use left, right, top, bottom, center, or maximize",
            other
        )),
    }
}

pub fn snap_active_window(direction: &str) -> Result<WindowBounds> {
    platform_snap_active_window(direction)
}

#[cfg(windows)]
fn platform_snap_active_window(direction: &str) -> Result<WindowBounds> {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
    };

    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_null() {
            return Err(anyhow!("no foreground window"));
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return Err(anyhow!("no monitor for foreground window"));
        }

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };

        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err(anyhow!("failed to read monitor info"));
        }

        let work = info.rcWork;
        let bounds = plan_snap_window(
            WindowBounds {
                x: work.left,
                y: work.top,
                width: work.right - work.left,
                height: work.bottom - work.top,
            },
            direction,
        )?;

        if SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ) == 0
        {
            return Err(anyhow!("failed to move foreground window"));
        }

        Ok(bounds)
    }
}

#[cfg(not(windows))]
fn platform_snap_active_window(_direction: &str) -> Result<WindowBounds> {
    Err(anyhow!(
        "active window snapping is only available on Windows"
    ))
}

fn run_builtin_job(job: &BackgroundJob) -> Result<Value> {
    match job.kind.as_str() {
        "folder_summary" => summarize_folder_job(&job.input_json),
        "batch_rename_preview" => batch_rename_preview_job(&job.input_json),
        other => Err(anyhow!("unsupported job kind: {}", other)),
    }
}

fn summarize_folder_job(input: &Value) -> Result<Value> {
    let folder = input
        .get("folder")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("folder_summary requires input_json.folder"))?;
    let mut files = 0usize;
    let mut directories = 0usize;
    let mut bytes = 0u64;

    for entry in fs::read_dir(folder).with_context(|| format!("read folder {}", folder))? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            directories += 1;
        } else if metadata.is_file() {
            files += 1;
            bytes += metadata.len();
        }
    }

    Ok(json!({
        "folder": folder,
        "files": files,
        "directories": directories,
        "bytes": bytes
    }))
}

fn batch_rename_preview_job(input: &Value) -> Result<Value> {
    let folder = input
        .get("folder")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("batch_rename_preview requires input_json.folder"))?;
    let prefix = input
        .get("prefix")
        .and_then(Value::as_str)
        .unwrap_or("renamed");
    let mut files = Vec::new();

    for entry in fs::read_dir(folder).with_context(|| format!("read folder {}", folder))? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    files.sort();

    let preview: Vec<Value> = files
        .into_iter()
        .enumerate()
        .map(|(index, from)| {
            let extension = std::path::Path::new(&from)
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| format!(".{}", value))
                .unwrap_or_default();
            json!({
                "from": from,
                "to": format!("{}-{:03}{}", prefix, index + 1, extension)
            })
        })
        .collect();

    Ok(json!({
        "folder": folder,
        "preview": preview,
        "applies_changes": false
    }))
}

fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("parse RFC3339 timestamp {}", value))?
        .with_timezone(&Utc))
}

fn validate_trigger(trigger: &str) -> Result<()> {
    if trigger.trim().is_empty() {
        return Err(anyhow!("snippet trigger is required"));
    }
    if trigger.contains(char::is_whitespace) {
        return Err(anyhow!("snippet trigger cannot contain whitespace"));
    }
    Ok(())
}

fn stable_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn looks_secret(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("begin private key")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("password=")
        || lower.contains("secret=")
        || lower.contains("token=")
}

fn alias_terms(process_name: &str, exe_path: &str, window_title: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for value in [process_name, exe_path, window_title] {
        for term in value
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|term| term.len() > 2)
        {
            let term = term.to_lowercase();
            if !terms.contains(&term) {
                terms.push(term);
            }
        }
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_window_rejects_unknown_direction() {
        let err = plan_snap_window(
            WindowBounds {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
            },
            "diagonal",
        )
        .expect_err("unknown direction should fail");

        assert!(err.to_string().contains("unknown snap direction"));
    }

    #[test]
    fn secret_clipboard_values_are_redacted() {
        assert!(looks_secret("TOKEN=abc123"));
        assert!(!looks_secret("normal clipboard text"));
    }
}
