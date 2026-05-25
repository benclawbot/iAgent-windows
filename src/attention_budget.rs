use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AttentionStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionState {
    #[serde(default)]
    pub settings: AttentionBudgetSettings,
    #[serde(default)]
    pub events: Vec<AttentionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionBudgetSettings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_hourly_cap")]
    pub max_interruptions_per_hour: u32,
    #[serde(default = "default_daily_cap")]
    pub max_interruptions_per_day: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_hours_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_hours_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snoozed_until: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snooze_reason: Option<String>,
    #[serde(default)]
    pub critical_kinds: Vec<String>,
}

impl Default for AttentionBudgetSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_interruptions_per_hour: default_hourly_cap(),
            max_interruptions_per_day: default_daily_cap(),
            quiet_hours_start: None,
            quiet_hours_end: None,
            snoozed_until: None,
            snooze_reason: None,
            critical_kinds: vec![
                "approval_needed".to_string(),
                "security".to_string(),
                "failure".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AttentionBudgetSettingsInput {
    pub enabled: Option<bool>,
    pub max_interruptions_per_hour: Option<u32>,
    pub max_interruptions_per_day: Option<u32>,
    pub quiet_hours_start: Option<Option<String>>,
    pub quiet_hours_end: Option<Option<String>>,
    pub critical_kinds: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionPreflightRequest {
    pub kind: String,
    pub title: String,
    pub priority: String,
    pub source: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionDecision {
    pub allowed: bool,
    pub reason: String,
    pub delivery: String,
    pub hourly_used: u32,
    pub hourly_limit: u32,
    pub daily_used: u32,
    pub daily_limit: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snoozed_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionEventInput {
    pub kind: String,
    pub title: String,
    pub priority: String,
    pub source: String,
    pub delivered: bool,
    pub delivery: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionEvent {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub priority: String,
    pub source: String,
    pub delivered: bool,
    pub delivery: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionSnoozeRequest {
    pub until: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionSnooze {
    pub until: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttentionDigest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub delivered_count: usize,
    pub deferred_count: usize,
    pub items: Vec<AttentionEvent>,
}

impl AttentionStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::jcode_dir()?.join("attention");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("budget.json"),
        })
    }

    pub fn state(&self) -> Result<AttentionState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read attention budget at {}", self.path.display()))
        } else {
            Ok(AttentionState::default())
        }
    }

    fn save_state(&self, state: &AttentionState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write attention budget at {}", self.path.display()))
    }

    pub fn settings(&self) -> Result<AttentionBudgetSettings> {
        Ok(self.state()?.settings)
    }

    pub fn update_settings(
        &self,
        input: AttentionBudgetSettingsInput,
    ) -> Result<AttentionBudgetSettings> {
        let mut state = self.state()?;
        if let Some(enabled) = input.enabled {
            state.settings.enabled = enabled;
        }
        if let Some(limit) = input.max_interruptions_per_hour {
            state.settings.max_interruptions_per_hour = limit.max(1);
        }
        if let Some(limit) = input.max_interruptions_per_day {
            state.settings.max_interruptions_per_day = limit.max(1);
        }
        if let Some(start) = input.quiet_hours_start {
            validate_optional_time(&start)?;
            state.settings.quiet_hours_start = start;
        }
        if let Some(end) = input.quiet_hours_end {
            validate_optional_time(&end)?;
            state.settings.quiet_hours_end = end;
        }
        if let Some(kinds) = input.critical_kinds {
            state.settings.critical_kinds = kinds
                .into_iter()
                .map(|kind| kind.trim().to_string())
                .filter(|kind| !kind.is_empty())
                .collect();
        }
        let settings = state.settings.clone();
        self.save_state(&state)?;
        Ok(settings)
    }

    pub fn preflight(&self, request: AttentionPreflightRequest) -> Result<AttentionDecision> {
        require_text("kind", &request.kind)?;
        require_text("title", &request.title)?;
        require_text("priority", &request.priority)?;
        require_text("source", &request.source)?;
        let at = parse_time(&request.at)?;
        let state = self.state()?;
        let settings = state.settings;
        let hourly_used = delivered_count_since(&state.events, at - chrono::Duration::hours(1), at);
        let daily_used = delivered_count_since(&state.events, at - chrono::Duration::days(1), at);
        let is_critical = is_critical(&settings, &request.kind, &request.priority);
        let base = |allowed: bool, reason: &str, delivery: &str| AttentionDecision {
            allowed,
            reason: reason.to_string(),
            delivery: delivery.to_string(),
            hourly_used,
            hourly_limit: settings.max_interruptions_per_hour,
            daily_used,
            daily_limit: settings.max_interruptions_per_day,
            snoozed_until: settings.snoozed_until,
        };

        if !settings.enabled {
            return Ok(base(false, "disabled", "digest"));
        }
        if is_critical {
            return Ok(base(true, "critical_interrupt_allowed", "immediate"));
        }
        if settings
            .snoozed_until
            .map(|until| until > at)
            .unwrap_or(false)
        {
            return Ok(base(false, "snoozed", "digest"));
        }
        if in_quiet_hours(
            at,
            settings.quiet_hours_start.as_deref(),
            settings.quiet_hours_end.as_deref(),
        )? {
            return Ok(base(false, "quiet_hours", "digest"));
        }
        if hourly_used >= settings.max_interruptions_per_hour {
            return Ok(base(false, "hourly_budget_exhausted", "digest"));
        }
        if daily_used >= settings.max_interruptions_per_day {
            return Ok(base(false, "daily_budget_exhausted", "digest"));
        }
        Ok(base(true, "within_budget", "immediate"))
    }

    pub fn record_event(&self, input: AttentionEventInput) -> Result<AttentionEvent> {
        require_text("kind", &input.kind)?;
        require_text("title", &input.title)?;
        require_text("priority", &input.priority)?;
        require_text("source", &input.source)?;
        let event = AttentionEvent {
            id: Uuid::new_v4().to_string(),
            kind: input.kind,
            title: input.title,
            priority: input.priority,
            source: input.source,
            delivered: input.delivered,
            delivery: input.delivery,
            occurred_at: parse_time(&input.occurred_at)?,
        };
        let mut state = self.state()?;
        state.events.insert(0, event.clone());
        self.save_state(&state)?;
        Ok(event)
    }

    pub fn snooze(&self, request: AttentionSnoozeRequest) -> Result<AttentionSnooze> {
        let until = parse_time(&request.until)?;
        let mut state = self.state()?;
        state.settings.snoozed_until = Some(until);
        state.settings.snooze_reason = request.reason.clone();
        self.save_state(&state)?;
        Ok(AttentionSnooze {
            until,
            reason: request.reason,
        })
    }

    pub fn resume(&self) -> Result<AttentionBudgetSettings> {
        let mut state = self.state()?;
        state.settings.snoozed_until = None;
        state.settings.snooze_reason = None;
        let settings = state.settings.clone();
        self.save_state(&state)?;
        Ok(settings)
    }

    pub fn digest(&self, from: &str, to: &str) -> Result<AttentionDigest> {
        let from = parse_time(from)?;
        let to = parse_time(to)?;
        let items: Vec<_> = self
            .state()?
            .events
            .into_iter()
            .filter(|event| event.occurred_at >= from && event.occurred_at < to)
            .collect();
        let delivered_count = items.iter().filter(|event| event.delivered).count();
        let deferred_count = items.iter().filter(|event| !event.delivered).count();
        Ok(AttentionDigest {
            from,
            to,
            delivered_count,
            deferred_count,
            items,
        })
    }

    pub fn history(&self, limit: usize) -> Result<Vec<AttentionEvent>> {
        Ok(self
            .state()?
            .events
            .into_iter()
            .take(limit.max(1))
            .collect())
    }
}

fn default_enabled() -> bool {
    true
}

fn default_hourly_cap() -> u32 {
    4
}

fn default_daily_cap() -> u32 {
    16
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn validate_optional_time(value: &Option<String>) -> Result<()> {
    if let Some(value) = value {
        parse_hour_minute(value)?;
    }
    Ok(())
}

fn parse_hour_minute(value: &str) -> Result<u32> {
    let (hour, minute) = value
        .split_once(':')
        .ok_or_else(|| anyhow!("quiet hour must use HH:MM"))?;
    let hour: u32 = hour.parse()?;
    let minute: u32 = minute.parse()?;
    if hour > 23 || minute > 59 {
        return Err(anyhow!("quiet hour must be within 00:00-23:59"));
    }
    Ok(hour * 60 + minute)
}

fn in_quiet_hours(at: DateTime<Utc>, start: Option<&str>, end: Option<&str>) -> Result<bool> {
    let (Some(start), Some(end)) = (start, end) else {
        return Ok(false);
    };
    let start = parse_hour_minute(start)?;
    let end = parse_hour_minute(end)?;
    let now = at.hour() * 60 + at.minute();
    if start < end {
        Ok(now >= start && now < end)
    } else {
        Ok(now >= start || now < end)
    }
}

fn delivered_count_since(events: &[AttentionEvent], from: DateTime<Utc>, to: DateTime<Utc>) -> u32 {
    events
        .iter()
        .filter(|event| event.delivered)
        .filter(|event| event.occurred_at >= from && event.occurred_at <= to)
        .count() as u32
}

fn is_critical(settings: &AttentionBudgetSettings, kind: &str, priority: &str) -> bool {
    priority.eq_ignore_ascii_case("critical")
        || settings
            .critical_kinds
            .iter()
            .any(|critical| critical.eq_ignore_ascii_case(kind))
}

fn require_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{label} is required"));
    }
    Ok(())
}
