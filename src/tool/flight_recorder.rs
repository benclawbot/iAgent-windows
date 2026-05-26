use super::{Tool, ToolContext, ToolOutput};
use crate::safety::{FlightRecorderQuery, PolicyDisposition, RiskLevel, SafetySystem};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct FlightRecorderTool;

impl FlightRecorderTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct FlightRecorderInput {
    action: String,
    limit: Option<usize>,
    action_query: Option<String>,
    risk_level: Option<String>,
    disposition: Option<String>,
    include_context: Option<bool>,
}

#[async_trait]
impl Tool for FlightRecorderTool {
    fn name(&self) -> &str {
        "flight_recorder"
    }

    fn description(&self) -> &str {
        "Inspect the Action Flight Recorder: a read-only timeline of actions, approvals, audit entries, evidence, and pending follow-ups."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["view"],
                    "description": "Flight recorder action."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of entries to return."
                },
                "action_query": {
                    "type": "string",
                    "description": "Case-insensitive filter over entry kind, action type, and summary."
                },
                "risk_level": {
                    "type": "string",
                    "enum": ["read_only", "edit_local", "external_send", "financial_legal", "destructive"],
                    "description": "Optional risk-level filter."
                },
                "disposition": {
                    "type": "string",
                    "enum": ["auto_allow", "confirm", "deny", "escalate"],
                    "description": "Optional policy-disposition filter."
                },
                "include_context": {
                    "type": "boolean",
                    "description": "Include stored structured context/evidence payloads."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: FlightRecorderInput = serde_json::from_value(input)?;
        if input.action != "view" {
            return Err(anyhow!(
                "unsupported flight_recorder action '{}'",
                input.action
            ));
        }

        let query = FlightRecorderQuery {
            limit: input.limit,
            action_query: input.action_query,
            risk_level: input
                .risk_level
                .as_deref()
                .map(parse_risk_level)
                .transpose()?,
            disposition: input
                .disposition
                .as_deref()
                .map(parse_disposition)
                .transpose()?,
            include_context: input.include_context.unwrap_or(false),
        };
        let view = SafetySystem::new().flight_recorder(query);
        let title = format!(
            "{} flight recorder entries ({} pending)",
            view.totals.total_entries, view.totals.pending_permissions
        );

        Ok(ToolOutput::new(serde_json::to_string_pretty(&view)?)
            .with_title(title)
            .with_metadata(json!({ "flight_recorder": view })))
    }
}

fn parse_risk_level(value: &str) -> Result<RiskLevel> {
    match value {
        "read_only" => Ok(RiskLevel::ReadOnly),
        "edit_local" => Ok(RiskLevel::EditLocal),
        "external_send" => Ok(RiskLevel::ExternalSend),
        "financial_legal" => Ok(RiskLevel::FinancialLegal),
        "destructive" => Ok(RiskLevel::Destructive),
        _ => Err(anyhow!("unsupported risk_level '{}'", value)),
    }
}

fn parse_disposition(value: &str) -> Result<PolicyDisposition> {
    match value {
        "auto_allow" => Ok(PolicyDisposition::AutoAllow),
        "confirm" => Ok(PolicyDisposition::Confirm),
        "deny" => Ok(PolicyDisposition::Deny),
        "escalate" => Ok(PolicyDisposition::Escalate),
        _ => Err(anyhow!("unsupported disposition '{}'", value)),
    }
}
