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
    pub settings: PersonalSettings,
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
    #[serde(default)]
    pub layouts: Vec<SavedWindowLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalSettings {
    #[serde(default = "default_true")]
    pub clipboard_history_enabled: bool,
    #[serde(default = "default_true")]
    pub reminder_notifications_enabled: bool,
    #[serde(default = "default_true")]
    pub background_jobs_enabled: bool,
    #[serde(default = "default_true")]
    pub proactive_suggestions_enabled: bool,
    #[serde(default = "default_true")]
    pub snippet_expansion_enabled: bool,
    #[serde(default = "default_max_clipboard_entries")]
    pub max_clipboard_entries: usize,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

impl Default for PersonalSettings {
    fn default() -> Self {
        Self {
            clipboard_history_enabled: true,
            reminder_notifications_enabled: true,
            background_jobs_enabled: true,
            proactive_suggestions_enabled: true,
            snippet_expansion_enabled: true,
            max_clipboard_entries: default_max_clipboard_entries(),
            retention_days: default_retention_days(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PersonalSettingsInput {
    pub clipboard_history_enabled: Option<bool>,
    pub reminder_notifications_enabled: Option<bool>,
    pub background_jobs_enabled: Option<bool>,
    pub proactive_suggestions_enabled: Option<bool>,
    pub snippet_expansion_enabled: Option<bool>,
    pub max_clipboard_entries: Option<usize>,
    pub retention_days: Option<u32>,
}

impl From<PersonalSettings> for PersonalSettingsInput {
    fn from(settings: PersonalSettings) -> Self {
        Self {
            clipboard_history_enabled: Some(settings.clipboard_history_enabled),
            reminder_notifications_enabled: Some(settings.reminder_notifications_enabled),
            background_jobs_enabled: Some(settings.background_jobs_enabled),
            proactive_suggestions_enabled: Some(settings.proactive_suggestions_enabled),
            snippet_expansion_enabled: Some(settings.snippet_expansion_enabled),
            max_clipboard_entries: Some(settings.max_clipboard_entries),
            retention_days: Some(settings.retention_days),
        }
    }
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
pub struct SnippetExpansion {
    pub trigger: String,
    pub replacement: String,
    pub output_text: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowPlacement {
    pub label: String,
    pub bounds: WindowBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedWindowLayout {
    pub id: String,
    pub name: String,
    pub placements: Vec<WindowPlacement>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SavedWindowLayoutInput {
    pub name: String,
    pub placements: Vec<WindowPlacement>,
}

#[derive(Debug, Clone)]
pub struct RuntimeTickInput {
    pub as_of: String,
    pub clipboard_content: Option<String>,
    pub active_app: Option<String>,
    pub active_window_title: Option<String>,
    pub run_one_job: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeTick {
    pub due_reminders: Vec<Reminder>,
    pub captured_clipboard: Option<ClipboardEntry>,
    pub completed_job: Option<BackgroundJob>,
    pub suggestions: Vec<ProactiveSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProactiveSuggestion {
    pub kind: String,
    pub title: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClearPersonalData {
    pub clipboard: bool,
    pub reminders: bool,
    pub snippets: bool,
    pub jobs: bool,
    pub app_windows: bool,
    pub layouts: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClearPersonalDataResult {
    pub clipboard: usize,
    pub reminders: usize,
    pub snippets: usize,
    pub jobs: usize,
    pub app_windows: usize,
    pub layouts: usize,
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

    pub fn settings(&self) -> Result<PersonalSettings> {
        Ok(self.state()?.settings)
    }

    pub fn update_settings(&self, input: PersonalSettingsInput) -> Result<PersonalSettings> {
        let mut state = self.state()?;
        let settings = &mut state.settings;

        if let Some(value) = input.clipboard_history_enabled {
            settings.clipboard_history_enabled = value;
        }
        if let Some(value) = input.reminder_notifications_enabled {
            settings.reminder_notifications_enabled = value;
        }
        if let Some(value) = input.background_jobs_enabled {
            settings.background_jobs_enabled = value;
        }
        if let Some(value) = input.proactive_suggestions_enabled {
            settings.proactive_suggestions_enabled = value;
        }
        if let Some(value) = input.snippet_expansion_enabled {
            settings.snippet_expansion_enabled = value;
        }
        if let Some(value) = input.max_clipboard_entries {
            settings.max_clipboard_entries = value.clamp(1, 250);
        }
        if let Some(value) = input.retention_days {
            settings.retention_days = value.clamp(1, 3650);
        }

        let settings = settings.clone();
        self.save_state(&state)?;
        Ok(settings)
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

    pub fn expand_typed_snippet(
        &self,
        input_text: &str,
        app_name: Option<&str>,
    ) -> Result<Option<SnippetExpansion>> {
        let state = self.state()?;
        if !state.settings.snippet_expansion_enabled {
            return Ok(None);
        }

        let app_name = app_name.unwrap_or_default().to_lowercase();
        let mut snippets = state.snippets;
        snippets.sort_by(|a, b| b.trigger.len().cmp(&a.trigger.len()));

        for snippet in snippets {
            if !snippet.enabled || !input_text.ends_with(&snippet.trigger) {
                continue;
            }
            if !snippet.app_scope.is_empty()
                && !snippet
                    .app_scope
                    .iter()
                    .any(|scope| app_name.contains(&scope.to_lowercase()))
            {
                continue;
            }

            let prefix = &input_text[..input_text.len() - snippet.trigger.len()];
            return Ok(Some(SnippetExpansion {
                trigger: snippet.trigger,
                replacement: snippet.body.clone(),
                output_text: format!("{}{}", prefix, snippet.body),
            }));
        }

        Ok(None)
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

    pub fn list_due_reminders(&self, as_of: &str) -> Result<Vec<Reminder>> {
        let as_of = parse_rfc3339_utc(as_of)?;
        let mut reminders: Vec<_> = self
            .state()?
            .reminders
            .into_iter()
            .filter(|reminder| {
                reminder.status == "pending"
                    && reminder.snoozed_until.unwrap_or(reminder.due_at) <= as_of
            })
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
        if !state.settings.clipboard_history_enabled {
            return Ok(None);
        }
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
        let max_entries = state
            .settings
            .max_clipboard_entries
            .max(1)
            .min(MAX_CLIPBOARD_ENTRIES.max(250));
        state.clipboard.truncate(max_entries);
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

    pub fn pin_clipboard(&self, id: &str, pinned: bool) -> Result<bool> {
        let mut state = self.state()?;
        let mut found = false;
        for entry in &mut state.clipboard {
            if entry.id == id {
                entry.pinned = pinned;
                found = true;
            }
        }
        if found {
            self.save_state(&state)?;
        }
        Ok(found)
    }

    pub fn delete_clipboard(&self, id: &str) -> Result<bool> {
        let mut state = self.state()?;
        let before = state.clipboard.len();
        state.clipboard.retain(|entry| entry.id != id);
        let removed = state.clipboard.len() != before;
        if removed {
            self.save_state(&state)?;
        }
        Ok(removed)
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

    pub fn capture_active_window(&self) -> Result<Option<AppWindowRecord>> {
        let Some(context) = desktop_monitor::capture_window_context() else {
            return Ok(None);
        };

        Ok(Some(self.record_app_window(
            &context.app_name,
            "",
            &context.window_title,
        )?))
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

    pub fn switch_to_app_description(&self, query: &str) -> Result<Option<AppWindowRecord>> {
        let Some(record) = self.resolve_app_description(query)? else {
            return Ok(None);
        };
        focus_recorded_window(&record)?;
        Ok(Some(record))
    }

    pub fn tile_app_descriptions(
        &self,
        left_query: &str,
        right_query: &str,
    ) -> Result<Option<Vec<WindowPlacement>>> {
        let Some(left) = self.resolve_app_description(left_query)? else {
            return Ok(None);
        };
        let Some(right) = self.resolve_app_description(right_query)? else {
            return Ok(None);
        };
        tile_recorded_windows(&left, &right).map(Some)
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
        let log_path = self.write_job_log(updated)?;
        updated.log_path = Some(log_path.to_string_lossy().to_string());
        let job = updated.clone();
        self.save_state(&state)?;
        Ok(Some(job))
    }

    fn write_job_log(&self, job: &BackgroundJob) -> Result<PathBuf> {
        let log_dir = self
            .path
            .parent()
            .unwrap_or_else(|| self.path.as_path())
            .join("jobs");
        fs::create_dir_all(&log_dir)?;
        let log_path = log_dir.join(format!("{}.json", job.id));
        crate::storage::write_json(&log_path, job)
            .with_context(|| format!("write personal job log at {}", log_path.display()))?;
        Ok(log_path)
    }

    pub fn run_runtime_tick(&self, input: RuntimeTickInput) -> Result<RuntimeTick> {
        let settings = self.settings()?;
        if let (Some(app), Some(title)) = (&input.active_app, &input.active_window_title) {
            let _ = self.record_app_window(app, "", title);
        }

        let captured_clipboard = if settings.clipboard_history_enabled {
            match &input.clipboard_content {
                Some(content) => self.record_clipboard(ClipboardInput {
                    content: content.clone(),
                    source_app: input.active_app.clone(),
                })?,
                None => None,
            }
        } else {
            None
        };

        let due_reminders = if settings.reminder_notifications_enabled {
            self.list_due_reminders(&input.as_of)?
        } else {
            Vec::new()
        };

        let completed_job = if input.run_one_job && settings.background_jobs_enabled {
            self.run_next_job()?
        } else {
            None
        };

        let suggestions = if settings.proactive_suggestions_enabled {
            self.proactive_suggestions(&due_reminders, captured_clipboard.as_ref(), &input)
        } else {
            Vec::new()
        };

        Ok(RuntimeTick {
            due_reminders,
            captured_clipboard,
            completed_job,
            suggestions,
        })
    }

    fn proactive_suggestions(
        &self,
        due_reminders: &[Reminder],
        clipboard: Option<&ClipboardEntry>,
        input: &RuntimeTickInput,
    ) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();
        if input.active_app.is_some() && input.active_window_title.is_some() {
            suggestions.push(ProactiveSuggestion {
                kind: "remember_context".to_string(),
                title: "Offer to remember this context".to_string(),
                detail: input.active_window_title.clone(),
            });
        }
        if clipboard
            .and_then(|entry| entry.content_text.as_deref())
            .is_some_and(|text| text.len() > 80)
        {
            suggestions.push(ProactiveSuggestion {
                kind: "make_snippet".to_string(),
                title: "Offer to save copied text as a snippet".to_string(),
                detail: None,
            });
        }
        for reminder in due_reminders.iter().take(3) {
            suggestions.push(ProactiveSuggestion {
                kind: "show_reminder".to_string(),
                title: reminder.title.clone(),
                detail: reminder.source_title.clone(),
            });
        }
        suggestions
    }

    pub fn save_window_layout(&self, input: SavedWindowLayoutInput) -> Result<SavedWindowLayout> {
        if input.name.trim().is_empty() {
            return Err(anyhow!("layout name is required"));
        }
        if input.placements.is_empty() {
            return Err(anyhow!("layout placements are required"));
        }

        let mut state = self.state()?;
        let now = Utc::now();
        if let Some(existing) = state
            .layouts
            .iter_mut()
            .find(|layout| layout.name == input.name)
        {
            existing.placements = input.placements;
            existing.updated_at = now;
            let layout = existing.clone();
            self.save_state(&state)?;
            return Ok(layout);
        }

        let layout = SavedWindowLayout {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            placements: input.placements,
            created_at: now,
            updated_at: now,
        };
        state.layouts.push(layout.clone());
        self.save_state(&state)?;
        Ok(layout)
    }

    pub fn list_window_layouts(&self) -> Result<Vec<SavedWindowLayout>> {
        let mut layouts = self.state()?.layouts;
        layouts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(layouts)
    }

    pub fn saved_window_layout_plan(&self, name: &str) -> Result<Option<Vec<WindowPlacement>>> {
        Ok(self
            .state()?
            .layouts
            .into_iter()
            .find(|layout| layout.name == name || layout.id == name)
            .map(|layout| layout.placements))
    }

    pub fn clear_personal_data(&self, clear: ClearPersonalData) -> Result<ClearPersonalDataResult> {
        let mut state = self.state()?;
        let mut result = ClearPersonalDataResult::default();

        if clear.clipboard {
            result.clipboard = state.clipboard.len();
            state.clipboard.clear();
        }
        if clear.reminders {
            result.reminders = state.reminders.len();
            state.reminders.clear();
        }
        if clear.snippets {
            result.snippets = state.snippets.len();
            state.snippets.clear();
        }
        if clear.jobs {
            result.jobs = state.jobs.len();
            state.jobs.clear();
        }
        if clear.app_windows {
            result.app_windows = state.app_windows.len();
            state.app_windows.clear();
        }
        if clear.layouts {
            result.layouts = state.layouts.len();
            state.layouts.clear();
        }

        self.save_state(&state)?;
        Ok(result)
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

pub fn plan_tile_two_windows(monitor: WindowBounds) -> Result<Vec<WindowPlacement>> {
    Ok(vec![
        WindowPlacement {
            label: "left".to_string(),
            bounds: plan_snap_window(monitor, "left")?,
        },
        WindowPlacement {
            label: "right".to_string(),
            bounds: plan_snap_window(monitor, "right")?,
        },
    ])
}

pub fn snap_active_window(direction: &str) -> Result<WindowBounds> {
    platform_snap_active_window(direction)
}

pub fn focus_recorded_window(record: &AppWindowRecord) -> Result<()> {
    platform_focus_recorded_window(record)
}

pub fn tile_recorded_windows(
    left: &AppWindowRecord,
    right: &AppWindowRecord,
) -> Result<Vec<WindowPlacement>> {
    platform_tile_recorded_windows(left, right)
}

#[cfg(windows)]
fn platform_snap_active_window(direction: &str) -> Result<WindowBounds> {
    unsafe {
        let hwnd = foreground_window()?;
        let monitor = monitor_bounds_for_window(hwnd)?;
        let bounds = plan_snap_window(monitor, direction)?;
        move_window(hwnd, bounds, false)?;
        Ok(bounds)
    }
}

#[cfg(windows)]
fn platform_focus_recorded_window(record: &AppWindowRecord) -> Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SW_RESTORE, SetForegroundWindow, ShowWindow,
    };

    unsafe {
        let hwnd = find_window_for_record(record)?;
        ShowWindow(hwnd, SW_RESTORE);
        if SetForegroundWindow(hwnd) == 0 {
            return Err(anyhow!("failed to focus window '{}'", record.window_title));
        }
        Ok(())
    }
}

#[cfg(windows)]
fn platform_tile_recorded_windows(
    left: &AppWindowRecord,
    right: &AppWindowRecord,
) -> Result<Vec<WindowPlacement>> {
    unsafe {
        let left_hwnd = find_window_for_record(left)?;
        let right_hwnd = find_window_for_record(right)?;
        let monitor = monitor_bounds_for_window(left_hwnd)?;
        let placements = plan_tile_two_windows(monitor)?;
        move_window(left_hwnd, placements[0].bounds, true)?;
        move_window(right_hwnd, placements[1].bounds, true)?;
        Ok(placements)
    }
}

#[cfg(windows)]
unsafe fn foreground_window() -> Result<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return Err(anyhow!("no foreground window"));
    }
    Ok(hwnd)
}

#[cfg(windows)]
unsafe fn monitor_bounds_for_window(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> Result<WindowBounds> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return Err(anyhow!("no monitor for window"));
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

    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return Err(anyhow!("failed to read monitor info"));
    }

    let work = info.rcWork;
    Ok(WindowBounds {
        x: work.left,
        y: work.top,
        width: work.right - work.left,
        height: work.bottom - work.top,
    })
}

#[cfg(windows)]
unsafe fn move_window(
    hwnd: windows_sys::Win32::Foundation::HWND,
    bounds: WindowBounds,
    activate: bool,
) -> Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};

    let flags = if activate {
        SWP_NOZORDER
    } else {
        SWP_NOZORDER | SWP_NOACTIVATE
    };
    if unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            flags,
        )
    } == 0
    {
        return Err(anyhow!("failed to move window"));
    }
    Ok(())
}

#[cfg(windows)]
unsafe fn find_window_for_record(
    record: &AppWindowRecord,
) -> Result<windows_sys::Win32::Foundation::HWND> {
    let candidates = unsafe { enumerate_visible_windows()? };
    let title = record.window_title.trim().to_lowercase();
    let process = record.process_name.trim().to_lowercase();
    let aliases: Vec<String> = record
        .aliases
        .iter()
        .map(|alias| alias.to_lowercase())
        .collect();

    candidates
        .into_iter()
        .find(|candidate| {
            let candidate_title = candidate.title.to_lowercase();
            (!title.is_empty()
                && (candidate_title.contains(&title) || title.contains(&candidate_title)))
                || (!process.is_empty() && candidate_title.contains(&process))
                || aliases.iter().any(|alias| candidate_title.contains(alias))
        })
        .map(|candidate| candidate.hwnd)
        .ok_or_else(|| anyhow!("no visible window matched '{}'", record.window_title))
}

#[cfg(windows)]
#[derive(Debug)]
struct VisibleWindow {
    hwnd: windows_sys::Win32::Foundation::HWND,
    title: String,
}

#[cfg(windows)]
unsafe fn enumerate_visible_windows() -> Result<Vec<VisibleWindow>> {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    };

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let windows = unsafe { &mut *(lparam as *mut Vec<VisibleWindow>) };
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }

        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return 1;
        }

        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if copied <= 0 {
            return 1;
        }

        buf.truncate(copied as usize);
        let title = String::from_utf16_lossy(&buf);
        if !title.trim().is_empty() {
            windows.push(VisibleWindow { hwnd, title });
        }
        1
    }

    let mut windows = Vec::new();
    if unsafe { EnumWindows(Some(enum_proc), &mut windows as *mut _ as isize) } == 0 {
        return Err(anyhow!("failed to enumerate windows"));
    }
    Ok(windows)
}

#[cfg(not(windows))]
fn platform_snap_active_window(_direction: &str) -> Result<WindowBounds> {
    Err(anyhow!(
        "active window snapping is only available on Windows"
    ))
}

#[cfg(not(windows))]
fn platform_focus_recorded_window(_record: &AppWindowRecord) -> Result<()> {
    Err(anyhow!("window focusing is only available on Windows"))
}

#[cfg(not(windows))]
fn platform_tile_recorded_windows(
    _left: &AppWindowRecord,
    _right: &AppWindowRecord,
) -> Result<Vec<WindowPlacement>> {
    Err(anyhow!("window tiling is only available on Windows"))
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

fn default_true() -> bool {
    true
}

fn default_max_clipboard_entries() -> usize {
    MAX_CLIPBOARD_ENTRIES
}

fn default_retention_days() -> u32 {
    30
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
