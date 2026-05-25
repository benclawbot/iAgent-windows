use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::personal_layer::{
    BackgroundJob, ClipboardEntry, PersonalSettings, PersonalStore, ProactiveSuggestion, Reminder,
    RuntimeTickInput,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonConfig {
    pub tick_interval_seconds: u64,
    pub run_jobs: bool,
    pub capture_clipboard: bool,
    pub capture_active_window: bool,
    pub headless: bool,
}

impl Default for PersonalDaemonConfig {
    fn default() -> Self {
        Self {
            tick_interval_seconds: 15,
            run_jobs: true,
            capture_clipboard: true,
            capture_active_window: true,
            headless: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonSnapshot {
    pub clipboard_content: Option<String>,
    pub active_app: Option<String>,
    pub active_window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonalDaemonTick {
    pub due_reminders: Vec<Reminder>,
    pub captured_clipboard: Option<ClipboardEntry>,
    pub completed_job: Option<BackgroundJob>,
    pub suggestions: Vec<ProactiveSuggestion>,
    pub notifications: Vec<PersonalDaemonNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonNotification {
    pub title: String,
    pub body: String,
    pub urgency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonStatus {
    pub settings: PersonalSettings,
    pub pending_reminders: usize,
    pub pending_jobs: usize,
    pub recent_clipboard_entries: usize,
    pub recent_app_windows: usize,
    pub saved_layouts: usize,
    pub timeline_entries: usize,
    pub project_workspaces: usize,
}

pub fn personal_daemon_status(store: &PersonalStore) -> Result<PersonalDaemonStatus> {
    let settings = store.settings()?;
    let pending_reminders = store.list_pending_reminders()?.len();
    let pending_jobs = store
        .list_jobs()?
        .into_iter()
        .filter(|job| job.status == "pending" || job.status == "running")
        .count();
    let recent_clipboard_entries = store
        .recent_clipboard(settings.max_clipboard_entries)?
        .len();
    let recent_app_windows = store.list_recent_app_windows(100)?.len();
    let saved_layouts = store.list_window_layouts()?.len();
    let panel = store.control_panel_summary()?;

    Ok(PersonalDaemonStatus {
        settings,
        pending_reminders,
        pending_jobs,
        recent_clipboard_entries,
        recent_app_windows,
        saved_layouts,
        timeline_entries: panel.timeline_entries,
        project_workspaces: panel.project_workspaces,
    })
}

pub fn run_personal_daemon_tick(
    store: &PersonalStore,
    snapshot: PersonalDaemonSnapshot,
    run_one_job: bool,
) -> Result<PersonalDaemonTick> {
    let tick = store.run_runtime_tick(RuntimeTickInput {
        as_of: chrono::Utc::now().to_rfc3339(),
        clipboard_content: snapshot.clipboard_content,
        active_app: snapshot.active_app,
        active_window_title: snapshot.active_window_title,
        run_one_job,
    })?;

    let mut notifications = Vec::new();
    for reminder in &tick.due_reminders {
        notifications.push(PersonalDaemonNotification {
            title: format!("Reminder: {}", reminder.title),
            body: reminder
                .note
                .clone()
                .or_else(|| reminder.source_title.clone())
                .unwrap_or_else(|| "A contextual reminder is due.".to_string()),
            urgency: "high".to_string(),
        });
    }

    if let Some(job) = &tick.completed_job {
        notifications.push(PersonalDaemonNotification {
            title: format!("Background job {}", job.status),
            body: format!("{} [{}]", job.description, job.kind),
            urgency: if job.status == "failed" {
                "high".to_string()
            } else {
                "normal".to_string()
            },
        });
    }

    for suggestion in &tick.suggestions {
        notifications.push(PersonalDaemonNotification {
            title: suggestion.title.clone(),
            body: suggestion.detail.clone().unwrap_or_default(),
            urgency: "low".to_string(),
        });
    }

    Ok(PersonalDaemonTick {
        due_reminders: tick.due_reminders,
        captured_clipboard: tick.captured_clipboard,
        completed_job: tick.completed_job,
        suggestions: tick.suggestions,
        notifications,
    })
}

pub async fn run_personal_daemon(config: PersonalDaemonConfig) -> Result<()> {
    let store = PersonalStore::load()?;
    let mut interval =
        tokio::time::interval(Duration::from_secs(config.tick_interval_seconds.max(1)));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                let snapshot = capture_snapshot(&config);
                let tick = run_personal_daemon_tick(&store, snapshot, config.run_jobs)?;
                emit_notifications(&tick, config.headless);
            }
        }
    }

    Ok(())
}

pub fn capture_snapshot(config: &PersonalDaemonConfig) -> PersonalDaemonSnapshot {
    let mut snapshot = PersonalDaemonSnapshot::default();

    if config.capture_clipboard {
        snapshot.clipboard_content = read_clipboard_text();
    }

    if config.capture_active_window {
        if let Some(context) = desktop_monitor::capture_window_context() {
            snapshot.active_app = Some(context.app_name);
            snapshot.active_window_title = Some(context.window_title);
        }
    }

    snapshot
}

fn read_clipboard_text() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

fn emit_notifications(tick: &PersonalDaemonTick, headless: bool) {
    if !headless {
        // Native toast/tray rendering is expected to subscribe to the same tick data.
        // The headless path keeps the first product daemon shippable in service mode.
        return;
    }

    for notification in &tick.notifications {
        println!(
            "[personal-daemon] {} [{}]: {}",
            notification.title, notification.urgency, notification.body
        );
    }
}
