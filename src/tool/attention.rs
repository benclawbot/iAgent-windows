use super::{Tool, ToolContext, ToolOutput};
use crate::attention_budget::{
    AttentionBudgetSettingsInput, AttentionEventInput, AttentionPreflightRequest,
    AttentionSnoozeRequest, AttentionStore,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct AttentionTool;

impl AttentionTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct AttentionInput {
    action: String,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    max_interruptions_per_hour: Option<u32>,
    #[serde(default)]
    max_interruptions_per_day: Option<u32>,
    #[serde(default)]
    quiet_hours_start: Option<String>,
    #[serde(default)]
    quiet_hours_end: Option<String>,
    #[serde(default)]
    clear_quiet_hours: bool,
    #[serde(default)]
    critical_kinds: Option<Vec<String>>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    at: Option<String>,
    #[serde(default)]
    delivered: Option<bool>,
    #[serde(default)]
    delivery: Option<String>,
    #[serde(default)]
    occurred_at: Option<String>,
    #[serde(default)]
    until: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for AttentionTool {
    fn name(&self) -> &str {
        "attention"
    }

    fn description(&self) -> &str {
        "Manage the ambient attention budget: quiet hours, interruption caps, snooze/resume, preflight decisions, delivery history, and digests."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["get_settings", "update_settings", "preflight", "record", "snooze", "resume", "digest", "history"],
                    "description": "Attention budget action."
                },
                "enabled": {"type": "boolean"},
                "max_interruptions_per_hour": {"type": "integer"},
                "max_interruptions_per_day": {"type": "integer"},
                "quiet_hours_start": {"type": "string"},
                "quiet_hours_end": {"type": "string"},
                "clear_quiet_hours": {"type": "boolean"},
                "critical_kinds": {"type": "array", "items": {"type": "string"}},
                "kind": {"type": "string"},
                "title": {"type": "string"},
                "priority": {"type": "string"},
                "source": {"type": "string"},
                "at": {"type": "string"},
                "delivered": {"type": "boolean"},
                "delivery": {"type": "string"},
                "occurred_at": {"type": "string"},
                "until": {"type": "string"},
                "reason": {"type": "string"},
                "from": {"type": "string"},
                "to": {"type": "string"},
                "limit": {"type": "integer"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: AttentionInput = serde_json::from_value(input)?;
        let store = AttentionStore::load()?;

        match input.action.as_str() {
            "get_settings" => {
                let settings = store.settings()?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&settings)?)
                    .with_title("Attention settings".to_string()))
            }
            "update_settings" => {
                let settings = store.update_settings(AttentionBudgetSettingsInput {
                    enabled: input.enabled,
                    max_interruptions_per_hour: input.max_interruptions_per_hour,
                    max_interruptions_per_day: input.max_interruptions_per_day,
                    quiet_hours_start: if input.clear_quiet_hours {
                        Some(None)
                    } else {
                        input.quiet_hours_start.map(Some)
                    },
                    quiet_hours_end: if input.clear_quiet_hours {
                        Some(None)
                    } else {
                        input.quiet_hours_end.map(Some)
                    },
                    critical_kinds: input.critical_kinds,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&settings)?)
                    .with_title("Attention settings updated".to_string()))
            }
            "preflight" => {
                let decision = store.preflight(AttentionPreflightRequest {
                    kind: required(input.kind, "kind")?,
                    title: required(input.title, "title")?,
                    priority: input.priority.unwrap_or_else(|| "normal".to_string()),
                    source: required(input.source, "source")?,
                    at: required(input.at, "at")?,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&decision)?)
                    .with_title(if decision.allowed {
                        "Attention allowed".to_string()
                    } else {
                        "Attention deferred".to_string()
                    })
                    .with_metadata(json!({ "attention_decision": decision })))
            }
            "record" => {
                let event = store.record_event(AttentionEventInput {
                    kind: required(input.kind, "kind")?,
                    title: required(input.title, "title")?,
                    priority: input.priority.unwrap_or_else(|| "normal".to_string()),
                    source: required(input.source, "source")?,
                    delivered: input.delivered.unwrap_or(true),
                    delivery: input.delivery.unwrap_or_else(|| "immediate".to_string()),
                    occurred_at: required(input.occurred_at.or(input.at), "occurred_at")?,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&event)?)
                    .with_title(format!("Attention event {}", event.id)))
            }
            "snooze" => {
                let snooze = store.snooze(AttentionSnoozeRequest {
                    until: required(input.until, "until")?,
                    reason: input.reason,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&snooze)?)
                    .with_title(format!("Attention snoozed until {}", snooze.until)))
            }
            "resume" => {
                let settings = store.resume()?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&settings)?)
                    .with_title("Attention resumed".to_string()))
            }
            "digest" => {
                let digest =
                    store.digest(&required(input.from, "from")?, &required(input.to, "to")?)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&digest)?)
                    .with_title(format!("{} attention event(s)", digest.items.len())))
            }
            "history" => {
                let history = store.history(input.limit.unwrap_or(20))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&history)?)
                    .with_title(format!("{} attention event(s)", history.len())))
            }
            other => Err(anyhow!("unsupported attention action '{}'", other)),
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
