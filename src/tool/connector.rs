use super::{Tool, ToolContext, ToolOutput};
use crate::connector_packs::{
    ConnectorEvidenceInput, ConnectorGrantRequest, ConnectorPackStore, ConnectorWritePreflight,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct ConnectorTool;

impl ConnectorTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct ConnectorInput {
    action: String,
    #[serde(default)]
    connector_id: Option<String>,
    #[serde(default)]
    grant_id: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    actor: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
    #[serde(default)]
    operation: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    active_only: Option<bool>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for ConnectorTool {
    fn name(&self) -> &str {
        "connector"
    }

    fn description(&self) -> &str {
        "Inspect connector packs, manage explicit read/write scopes, preflight connector writes, and record run evidence for every approved write."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "grant_scope", "revoke_grant", "list_grants", "preflight_write", "record_write", "audit_writes"],
                    "description": "Connector pack action."
                },
                "connector_id": {"type": "string"},
                "grant_id": {"type": "string"},
                "scopes": {"type": "array", "items": {"type": "string"}},
                "actor": {"type": "string"},
                "reason": {"type": "string"},
                "expires_at": {"type": "string", "description": "Optional RFC3339 grant expiration."},
                "operation": {"type": "string"},
                "target": {"type": "string"},
                "run_id": {"type": "string"},
                "tool_call_id": {"type": "string"},
                "summary": {"type": "string"},
                "evidence_refs": {"type": "array", "items": {"type": "string"}},
                "active_only": {"type": "boolean"},
                "limit": {"type": "integer"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: ConnectorInput = serde_json::from_value(input)?;
        let store = ConnectorPackStore::load()?;

        match input.action.as_str() {
            "list" => {
                let catalog = ConnectorPackStore::built_in_catalog();
                Ok(ToolOutput::new(serde_json::to_string_pretty(&catalog)?)
                    .with_title(format!("{} connectors", catalog.len())))
            }
            "get" => {
                let connector = ConnectorPackStore::get_connector(&required(
                    input.connector_id,
                    "connector_id",
                )?)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&connector)?)
                    .with_title(connector.display_name))
            }
            "grant_scope" => {
                let grant = store.grant_scopes(ConnectorGrantRequest {
                    connector_id: required(input.connector_id, "connector_id")?,
                    scopes: input.scopes,
                    actor: required(input.actor, "actor")?,
                    reason: required(input.reason, "reason")?,
                    expires_at: input.expires_at,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&grant)?)
                    .with_title(format!("Connector scope grant {}", grant.id)))
            }
            "revoke_grant" => {
                let grant_id = required(input.grant_id, "grant_id")?;
                let revoked = store.revoke_grant(&grant_id)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&json!({
                    "grant_id": grant_id,
                    "revoked": revoked
                }))?)
                .with_title("Connector grant revoked".to_string()))
            }
            "list_grants" => {
                let grants = store.list_grants(
                    input.connector_id.as_deref(),
                    input.active_only.unwrap_or(true),
                )?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&grants)?)
                    .with_title(format!("{} connector grants", grants.len())))
            }
            "preflight_write" => {
                let decision = store.preflight_write(ConnectorWritePreflight {
                    connector_id: required(input.connector_id, "connector_id")?,
                    operation: required(input.operation, "operation")?,
                    target: required(input.target, "target")?,
                    run_id: input.run_id,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&decision)?)
                    .with_title(if decision.allowed {
                        "Connector write allowed".to_string()
                    } else {
                        "Connector write blocked".to_string()
                    })
                    .with_metadata(json!({ "connector_write_decision": decision })))
            }
            "record_write" => {
                let evidence = store.record_write_evidence(ConnectorEvidenceInput {
                    connector_id: required(input.connector_id, "connector_id")?,
                    operation: required(input.operation, "operation")?,
                    target: required(input.target, "target")?,
                    run_id: required(input.run_id, "run_id")?,
                    tool_call_id: input.tool_call_id,
                    summary: required(input.summary, "summary")?,
                    evidence_refs: input.evidence_refs,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&evidence)?)
                    .with_title(format!("Connector write evidence {}", evidence.id))
                    .with_metadata(json!({ "connector_write_evidence": evidence })))
            }
            "audit_writes" => {
                let entries = store.audit_writes(
                    input.connector_id.as_deref(),
                    input.run_id.as_deref(),
                    input.limit.unwrap_or(20),
                )?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&entries)?)
                    .with_title(format!("{} connector write records", entries.len())))
            }
            other => Err(anyhow!("unsupported connector action '{}'", other)),
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
