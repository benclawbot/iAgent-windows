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
    #[serde(default)]
    pub timeline: Vec<TimelineEntry>,
    #[serde(default)]
    pub workspaces: Vec<ProjectWorkspace>,
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
    #[serde(default = "default_true")]
    pub timeline_enabled: bool,
    #[serde(default = "default_true")]
    pub app_history_enabled: bool,
    #[serde(default)]
    pub screenshots_enabled: bool,
    #[serde(default)]
    pub ocr_enabled: bool,
    #[serde(default = "default_true")]
    pub uia_text_enabled: bool,
    #[serde(default = "default_true")]
    pub computer_use_enabled: bool,
    #[serde(default = "default_true")]
    pub prompt_injection_defense_enabled: bool,
    #[serde(default)]
    pub encrypted_sensitive_storage: bool,
    #[serde(default = "default_true")]
    pub require_approval_for_personal_actions: bool,
    #[serde(default)]
    pub excluded_apps: Vec<String>,
    #[serde(default)]
    pub private_title_patterns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_paused_until: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_pause_reason: Option<String>,
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
            timeline_enabled: true,
            app_history_enabled: true,
            screenshots_enabled: false,
            ocr_enabled: false,
            uia_text_enabled: true,
            computer_use_enabled: true,
            prompt_injection_defense_enabled: true,
            encrypted_sensitive_storage: false,
            require_approval_for_personal_actions: true,
            excluded_apps: Vec::new(),
            private_title_patterns: Vec::new(),
            capture_paused_until: None,
            capture_pause_reason: None,
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
    pub timeline_enabled: Option<bool>,
    pub app_history_enabled: Option<bool>,
    pub screenshots_enabled: Option<bool>,
    pub ocr_enabled: Option<bool>,
    pub uia_text_enabled: Option<bool>,
    pub computer_use_enabled: Option<bool>,
    pub prompt_injection_defense_enabled: Option<bool>,
    pub encrypted_sensitive_storage: Option<bool>,
    pub require_approval_for_personal_actions: Option<bool>,
    pub excluded_apps: Option<Vec<String>>,
    pub private_title_patterns: Option<Vec<String>>,
    pub capture_paused_until: Option<Option<DateTime<Utc>>>,
    pub capture_pause_reason: Option<Option<String>>,
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
            timeline_enabled: Some(settings.timeline_enabled),
            app_history_enabled: Some(settings.app_history_enabled),
            screenshots_enabled: Some(settings.screenshots_enabled),
            ocr_enabled: Some(settings.ocr_enabled),
            uia_text_enabled: Some(settings.uia_text_enabled),
            computer_use_enabled: Some(settings.computer_use_enabled),
            prompt_injection_defense_enabled: Some(settings.prompt_injection_defense_enabled),
            encrypted_sensitive_storage: Some(settings.encrypted_sensitive_storage),
            require_approval_for_personal_actions: Some(
                settings.require_approval_for_personal_actions,
            ),
            excluded_apps: Some(settings.excluded_apps),
            private_title_patterns: Some(settings.private_title_patterns),
            capture_paused_until: Some(settings.capture_paused_until),
            capture_pause_reason: Some(settings.capture_pause_reason),
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
    pub timeline: bool,
    pub workspaces: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClearPersonalDataResult {
    pub clipboard: usize,
    pub reminders: usize,
    pub snippets: usize,
    pub jobs: usize,
    pub app_windows: usize,
    pub layouts: usize,
    pub timeline: usize,
    pub workspaces: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensitiveFinding {
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensitiveContextPreview {
    pub redacted: bool,
    pub redacted_text: String,
    pub findings: Vec<SensitiveFinding>,
    pub blocked_by_exclusion: bool,
    pub capture_paused: bool,
    pub will_store_text: bool,
    pub will_store_screenshot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineEntry {
    pub id: String,
    pub observed_at: DateTime<Utc>,
    pub app_name: String,
    pub window_title: String,
    pub activity: String,
    pub text_excerpt: Option<String>,
    pub screenshot_path: Option<String>,
    pub source: String,
    #[serde(default)]
    pub capture_modes: Vec<String>,
    #[serde(default)]
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TimelineEntryInput {
    pub app_name: String,
    pub window_title: String,
    pub activity: String,
    pub text_excerpt: Option<String>,
    pub screenshot_path: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineSearch {
    pub query: Option<String>,
    pub app_name: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputerUseActionPlan {
    pub kind: String,
    pub target: Option<String>,
    pub value: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub struct ComputerUseRequest {
    pub goal: String,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub observation_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputerUsePlan {
    pub goal: String,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub actions: Vec<ComputerUseActionPlan>,
    pub verification_required: bool,
    pub permission_tier: String,
    #[serde(default)]
    pub risk_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectWorkspace {
    pub id: String,
    pub name: String,
    pub layout_name: Option<String>,
    #[serde(default)]
    pub app_queries: Vec<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProjectWorkspaceInput {
    pub name: String,
    pub layout_name: Option<String>,
    pub app_queries: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalControlPanelSummary {
    pub snippets: usize,
    pub reminders: usize,
    pub clipboard_entries: usize,
    pub jobs: usize,
    pub recent_app_windows: usize,
    pub saved_layouts: usize,
    pub timeline_entries: usize,
    pub project_workspaces: usize,
    pub privacy: PersonalPrivacySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalPrivacySummary {
    pub timeline_enabled: bool,
    pub screenshots_enabled: bool,
    pub ocr_enabled: bool,
    pub uia_text_enabled: bool,
    pub encrypted_sensitive_storage: bool,
    pub capture_paused: bool,
    pub capture_paused_until: Option<DateTime<Utc>>,
    pub capture_pause_reason: Option<String>,
    pub excluded_apps: Vec<String>,
    pub private_title_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensitiveContextStorageSummary {
    pub clipboard_entries: usize,
    pub timeline_entries: usize,
    pub recent_app_windows: usize,
    pub retained_days: u32,
    pub max_clipboard_entries: usize,
    pub redacted_clipboard_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SensitiveContextFirewallStatus {
    pub capture_paused: bool,
    pub capture_paused_until: Option<DateTime<Utc>>,
    pub pause_reason: Option<String>,
    pub storage: SensitiveContextStorageSummary,
    pub privacy: PersonalPrivacySummary,
}

impl PersonalStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::iagent_dir()?.join("personal");
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
        if let Some(value) = input.timeline_enabled {
            settings.timeline_enabled = value;
        }
        if let Some(value) = input.app_history_enabled {
            settings.app_history_enabled = value;
        }
        if let Some(value) = input.screenshots_enabled {
            settings.screenshots_enabled = value;
        }
        if let Some(value) = input.ocr_enabled {
            settings.ocr_enabled = value;
        }
        if let Some(value) = input.uia_text_enabled {
            settings.uia_text_enabled = value;
        }
        if let Some(value) = input.computer_use_enabled {
            settings.computer_use_enabled = value;
        }
        if let Some(value) = input.prompt_injection_defense_enabled {
            settings.prompt_injection_defense_enabled = value;
        }
        if let Some(value) = input.encrypted_sensitive_storage {
            settings.encrypted_sensitive_storage = value;
        }
        if let Some(value) = input.require_approval_for_personal_actions {
            settings.require_approval_for_personal_actions = value;
        }
        if let Some(value) = input.excluded_apps {
            settings.excluded_apps = normalize_list(value);
        }
        if let Some(value) = input.private_title_patterns {
            settings.private_title_patterns = normalize_list(value);
        }
        if let Some(value) = input.capture_paused_until {
            settings.capture_paused_until = value;
        }
        if let Some(value) = input.capture_pause_reason {
            settings.capture_pause_reason = value.filter(|reason| !reason.trim().is_empty());
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
        if !state.settings.clipboard_history_enabled
            || capture_paused(&state.settings)
            || input
                .source_app
                .as_deref()
                .is_some_and(|app| !should_observe_with_settings(&state.settings, app, ""))
        {
            return Ok(None);
        }
        if state.clipboard.iter().any(|entry| {
            entry.content_hash == hash && entry.content_text.as_deref() == Some(&input.content)
        }) {
            return Ok(None);
        }

        let preview = preview_sensitive_text(
            &input.content,
            &state.settings,
            input.source_app.as_deref(),
            None,
        );
        let entry = ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            captured_at: Utc::now(),
            content_hash: hash,
            content_text: if preview.redacted {
                None
            } else {
                Some(input.content)
            },
            source_app: input.source_app,
            pinned: false,
            redacted: preview.redacted,
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
        let mut state = self.state()?;
        if !state.settings.app_history_enabled
            || capture_paused(&state.settings)
            || !should_observe_with_settings(&state.settings, process_name, window_title)
        {
            return Err(anyhow!(
                "app/window capture is blocked by Sensitive Context Firewall"
            ));
        }
        let record = AppWindowRecord {
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            process_name: process_name.to_string(),
            exe_path: exe_path.to_string(),
            window_title: window_title.to_string(),
            aliases: alias_terms(process_name, exe_path, window_title),
        };
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
            .unwrap_or(self.path.as_path())
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

    pub fn should_observe_app(&self, app_name: &str, window_title: &str) -> Result<bool> {
        let settings = self.settings()?;
        Ok(should_observe_with_settings(
            &settings,
            app_name,
            window_title,
        ))
    }

    pub fn record_timeline_entry(
        &self,
        input: TimelineEntryInput,
    ) -> Result<Option<TimelineEntry>> {
        let mut state = self.state()?;
        if !state.settings.timeline_enabled
            || capture_paused(&state.settings)
            || !should_observe_with_settings(&state.settings, &input.app_name, &input.window_title)
        {
            return Ok(None);
        }
        let preview = input
            .text_excerpt
            .as_deref()
            .map(|text| {
                preview_sensitive_text(
                    text,
                    &state.settings,
                    Some(&input.app_name),
                    Some(&input.window_title),
                )
            })
            .unwrap_or_else(|| {
                preview_sensitive_text(
                    "",
                    &state.settings,
                    Some(&input.app_name),
                    Some(&input.window_title),
                )
            });

        let mut capture_modes = Vec::new();
        if state.settings.uia_text_enabled && input.text_excerpt.is_some() {
            capture_modes.push("uia_text".to_string());
        }
        if state.settings.ocr_enabled {
            capture_modes.push("ocr".to_string());
        }
        if state.settings.screenshots_enabled && input.screenshot_path.is_some() {
            capture_modes.push("screenshot".to_string());
        }

        let entry = TimelineEntry {
            id: Uuid::new_v4().to_string(),
            observed_at: Utc::now(),
            app_name: input.app_name,
            window_title: input.window_title,
            activity: input.activity,
            text_excerpt: input.text_excerpt.map(|text| {
                if preview.redacted {
                    truncate_chars(&preview.redacted_text, 500)
                } else {
                    truncate_chars(&text, 500)
                }
            }),
            screenshot_path: if state.settings.screenshots_enabled {
                input.screenshot_path
            } else {
                None
            },
            source: input.source,
            capture_modes,
            risk_flags: preview
                .findings
                .iter()
                .map(|finding| finding.kind.clone())
                .collect(),
        };

        state.timeline.insert(0, entry.clone());
        prune_timeline(&mut state);
        self.save_state(&state)?;
        Ok(Some(entry))
    }

    pub fn search_timeline(&self, search: TimelineSearch) -> Result<Vec<TimelineEntry>> {
        let query_terms: Vec<String> = search
            .query
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect();
        let app_filter = search.app_name.unwrap_or_default().to_lowercase();
        let limit = search.limit.max(1);

        Ok(self
            .state()?
            .timeline
            .into_iter()
            .filter(|entry| {
                app_filter.is_empty() || entry.app_name.to_lowercase().contains(&app_filter)
            })
            .filter(|entry| {
                if query_terms.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {} {}",
                    entry.app_name,
                    entry.window_title,
                    entry.activity,
                    entry.text_excerpt.as_deref().unwrap_or_default()
                )
                .to_lowercase();
                query_terms.iter().all(|term| haystack.contains(term))
            })
            .take(limit)
            .collect())
    }

    pub fn delete_timeline_entry(&self, id: &str) -> Result<bool> {
        let mut state = self.state()?;
        let before = state.timeline.len();
        state.timeline.retain(|entry| entry.id != id);
        let removed = state.timeline.len() != before;
        if removed {
            self.save_state(&state)?;
        }
        Ok(removed)
    }

    pub fn draft_computer_use_plan(&self, request: ComputerUseRequest) -> Result<ComputerUsePlan> {
        let settings = self.settings()?;
        if !settings.computer_use_enabled {
            return Err(anyhow!("computer-use action loop is disabled"));
        }

        let observation = request.observation_text.unwrap_or_default();
        let mut risk_flags = Vec::new();
        if settings.prompt_injection_defense_enabled && looks_like_prompt_injection(&observation) {
            risk_flags.push("prompt_injection".to_string());
        }
        if let (Some(app), Some(title)) = (&request.app_name, &request.window_title)
            && !should_observe_with_settings(&settings, app, title)
        {
            risk_flags.push("private_or_excluded_app".to_string());
        }

        let mut actions = vec![ComputerUseActionPlan {
            kind: "observe".to_string(),
            target: request.window_title.clone(),
            value: None,
            rationale: "Capture screenshot/UI tree before acting.".to_string(),
        }];
        actions.push(ComputerUseActionPlan {
            kind: "act".to_string(),
            target: request.app_name.clone(),
            value: Some(request.goal.clone()),
            rationale: "Execute bounded click/type/scroll/wait steps after approval.".to_string(),
        });
        actions.push(ComputerUseActionPlan {
            kind: "verify".to_string(),
            target: request.window_title.clone(),
            value: None,
            rationale: "Re-observe and confirm the requested state changed.".to_string(),
        });

        Ok(ComputerUsePlan {
            goal: request.goal,
            app_name: request.app_name,
            window_title: request.window_title,
            actions,
            verification_required: true,
            permission_tier: if risk_flags.is_empty()
                && !settings.require_approval_for_personal_actions
            {
                "auto".to_string()
            } else {
                "confirm".to_string()
            },
            risk_flags,
        })
    }

    pub fn save_project_workspace(&self, input: ProjectWorkspaceInput) -> Result<ProjectWorkspace> {
        if input.name.trim().is_empty() {
            return Err(anyhow!("workspace name is required"));
        }
        let mut state = self.state()?;
        let now = Utc::now();
        if let Some(existing) = state
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.name == input.name)
        {
            existing.layout_name = input.layout_name;
            existing.app_queries = normalize_list(input.app_queries);
            existing.notes = input.notes;
            existing.updated_at = now;
            let workspace = existing.clone();
            self.save_state(&state)?;
            return Ok(workspace);
        }

        let workspace = ProjectWorkspace {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            layout_name: input.layout_name,
            app_queries: normalize_list(input.app_queries),
            notes: input.notes,
            created_at: now,
            updated_at: now,
        };
        state.workspaces.push(workspace.clone());
        self.save_state(&state)?;
        Ok(workspace)
    }

    pub fn list_project_workspaces(&self) -> Result<Vec<ProjectWorkspace>> {
        let mut workspaces = self.state()?.workspaces;
        workspaces.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(workspaces)
    }

    pub fn control_panel_summary(&self) -> Result<PersonalControlPanelSummary> {
        let state = self.state()?;
        let pending_reminders = state
            .reminders
            .iter()
            .filter(|reminder| reminder.status == "pending")
            .count();
        let active_jobs = state
            .jobs
            .iter()
            .filter(|job| matches!(job.status.as_str(), "pending" | "running"))
            .count();
        Ok(PersonalControlPanelSummary {
            snippets: state.snippets.len(),
            reminders: pending_reminders,
            clipboard_entries: state.clipboard.len(),
            jobs: active_jobs,
            recent_app_windows: state.app_windows.len(),
            saved_layouts: state.layouts.len(),
            timeline_entries: state.timeline.len(),
            project_workspaces: state.workspaces.len(),
            privacy: privacy_summary(&state),
        })
    }

    pub fn sensitive_context_firewall_status(&self) -> Result<SensitiveContextFirewallStatus> {
        let state = self.state()?;
        let privacy = privacy_summary(&state);
        Ok(SensitiveContextFirewallStatus {
            capture_paused: privacy.capture_paused,
            capture_paused_until: privacy.capture_paused_until,
            pause_reason: privacy.capture_pause_reason.clone(),
            storage: SensitiveContextStorageSummary {
                clipboard_entries: state.clipboard.len(),
                timeline_entries: state.timeline.len(),
                recent_app_windows: state.app_windows.len(),
                retained_days: state.settings.retention_days,
                max_clipboard_entries: state.settings.max_clipboard_entries,
                redacted_clipboard_entries: state
                    .clipboard
                    .iter()
                    .filter(|entry| entry.redacted)
                    .count(),
            },
            privacy,
        })
    }

    pub fn preview_sensitive_context(
        &self,
        text: &str,
        app_name: Option<&str>,
        window_title: Option<&str>,
    ) -> Result<SensitiveContextPreview> {
        let settings = self.settings()?;
        Ok(preview_sensitive_text(
            text,
            &settings,
            app_name,
            window_title,
        ))
    }

    pub fn pause_sensitive_capture(
        &self,
        minutes: u32,
        reason: Option<String>,
    ) -> Result<SensitiveContextFirewallStatus> {
        let mut state = self.state()?;
        let minutes = minutes.clamp(1, 24 * 60);
        state.settings.capture_paused_until =
            Some(Utc::now() + chrono::Duration::minutes(minutes as i64));
        state.settings.capture_pause_reason = reason.filter(|value| !value.trim().is_empty());
        self.save_state(&state)?;
        self.sensitive_context_firewall_status()
    }

    pub fn resume_sensitive_capture(&self) -> Result<SensitiveContextFirewallStatus> {
        let mut state = self.state()?;
        state.settings.capture_paused_until = None;
        state.settings.capture_pause_reason = None;
        self.save_state(&state)?;
        self.sensitive_context_firewall_status()
    }

    pub fn forget_recent_context(&self, minutes: u32) -> Result<ClearPersonalDataResult> {
        let mut state = self.state()?;
        let cutoff = Utc::now() - chrono::Duration::minutes(minutes.clamp(1, 24 * 60) as i64);
        let mut result = ClearPersonalDataResult::default();

        let before = state.clipboard.len();
        state
            .clipboard
            .retain(|entry| entry.pinned || entry.captured_at < cutoff);
        result.clipboard = before - state.clipboard.len();

        let before = state.timeline.len();
        state.timeline.retain(|entry| entry.observed_at < cutoff);
        result.timeline = before - state.timeline.len();

        let before = state.app_windows.len();
        state.app_windows.retain(|entry| entry.observed_at < cutoff);
        result.app_windows = before - state.app_windows.len();

        self.save_state(&state)?;
        Ok(result)
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
        if clear.timeline {
            result.timeline = state.timeline.len();
            state.timeline.clear();
        }
        if clear.workspaces {
            result.workspaces = state.workspaces.len();
            state.workspaces.clear();
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

fn normalize_list(values: Vec<String>) -> Vec<String> {
    let mut values: Vec<String> = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
}

fn should_observe_with_settings(
    settings: &PersonalSettings,
    app_name: &str,
    window_title: &str,
) -> bool {
    let app = app_name.to_lowercase();
    let title = window_title.to_lowercase();
    if settings
        .excluded_apps
        .iter()
        .any(|excluded| app.contains(&excluded.to_lowercase()))
    {
        return false;
    }
    !settings
        .private_title_patterns
        .iter()
        .any(|pattern| title.contains(&pattern.to_lowercase()))
}

fn capture_paused(settings: &PersonalSettings) -> bool {
    settings
        .capture_paused_until
        .is_some_and(|until| until > Utc::now())
}

fn privacy_summary(state: &PersonalState) -> PersonalPrivacySummary {
    PersonalPrivacySummary {
        timeline_enabled: state.settings.timeline_enabled,
        screenshots_enabled: state.settings.screenshots_enabled,
        ocr_enabled: state.settings.ocr_enabled,
        uia_text_enabled: state.settings.uia_text_enabled,
        encrypted_sensitive_storage: state.settings.encrypted_sensitive_storage,
        capture_paused: capture_paused(&state.settings),
        capture_paused_until: state.settings.capture_paused_until,
        capture_pause_reason: state.settings.capture_pause_reason.clone(),
        excluded_apps: state.settings.excluded_apps.clone(),
        private_title_patterns: state.settings.private_title_patterns.clone(),
    }
}

fn preview_sensitive_text(
    text: &str,
    settings: &PersonalSettings,
    app_name: Option<&str>,
    window_title: Option<&str>,
) -> SensitiveContextPreview {
    let blocked_by_exclusion = app_name.is_some_and(|app| {
        !should_observe_with_settings(settings, app, window_title.unwrap_or_default())
    });
    let capture_paused = capture_paused(settings);
    let (redacted_text, findings) = redact_sensitive_text(text);
    let redacted = redacted_text != text;

    SensitiveContextPreview {
        redacted,
        redacted_text,
        findings,
        blocked_by_exclusion,
        capture_paused,
        will_store_text: !blocked_by_exclusion && !capture_paused,
        will_store_screenshot: !blocked_by_exclusion
            && !capture_paused
            && settings.screenshots_enabled,
    }
}

fn redact_sensitive_text(value: &str) -> (String, Vec<SensitiveFinding>) {
    let mut findings = Vec::new();
    let mut redacted = Vec::new();

    for token in value.split_whitespace() {
        let (kind, label) = classify_sensitive_token(token);
        if let Some(kind) = kind {
            if !findings
                .iter()
                .any(|finding: &SensitiveFinding| finding.kind == kind)
            {
                findings.push(SensitiveFinding {
                    kind: kind.to_string(),
                    label: label.unwrap_or(kind).to_string(),
                });
            }
            redacted.push(format!("[REDACTED:{kind}]"));
        } else {
            redacted.push(token.to_string());
        }
    }

    (redacted.join(" "), findings)
}

fn classify_sensitive_token(token: &str) -> (Option<&'static str>, Option<&'static str>) {
    let lower = token.to_lowercase();
    if lower.contains("api_key") || lower.contains("apikey") || lower.starts_with("sk-") {
        return (Some("api_key"), Some("API key"));
    }
    if lower.contains("password=") || lower.contains("passwd=") || lower.contains("pwd=") {
        return (Some("password"), Some("password"));
    }
    if lower.contains("secret=") || lower.contains("client_secret") {
        return (Some("secret"), Some("secret"));
    }
    if lower.contains("token=") || lower.contains("bearer ") {
        return (Some("token"), Some("token"));
    }
    if lower.contains('@') && lower.contains('.') {
        return (Some("email"), Some("email address"));
    }
    if lower.chars().filter(|ch| ch.is_ascii_digit()).count() >= 13
        && lower.chars().filter(|ch| ch.is_ascii_digit()).count() <= 19
    {
        return (Some("payment_card"), Some("payment card"));
    }
    (None, None)
}

fn prune_timeline(state: &mut PersonalState) {
    let max_entries = (state.settings.retention_days as usize).clamp(1, 3650) * 24;
    state.timeline.truncate(max_entries);
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn looks_like_prompt_injection(value: &str) -> bool {
    let value = value.to_lowercase();
    [
        "ignore previous instructions",
        "ignore all previous",
        "system prompt",
        "developer message",
        "do not ask",
        "without asking",
    ]
    .iter()
    .any(|needle| value.contains(needle))
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
    lower.contains("begin private key") || !redact_sensitive_text(value).1.is_empty()
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
