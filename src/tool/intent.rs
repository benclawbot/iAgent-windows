use super::{Tool, ToolContext, ToolOutput};
use crate::intent_manifest::{IntentActionPlanRequest, IntentManifestStore};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

pub struct IntentTool;

impl IntentTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct IntentInput {
    action: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    action_id: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    parameters: Option<Value>,
    #[serde(default)]
    max_depth: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for IntentTool {
    fn name(&self) -> &str {
        "intent"
    }

    fn description(&self) -> &str {
        "Discover, validate, import, list, and plan iagent.intent.json app action manifests without executing arbitrary local code."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["discover", "import", "list", "get", "plan"],
                    "description": "Intent manifest action."
                },
                "path": {"type": "string", "description": "Path to an iagent.intent.json file for import."},
                "root": {"type": "string", "description": "Root directory for manifest discovery."},
                "app_id": {"type": "string"},
                "action_id": {"type": "string"},
                "query": {"type": "string"},
                "parameters": {"type": "object", "additionalProperties": true},
                "max_depth": {"type": "integer"},
                "limit": {"type": "integer"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: IntentInput = serde_json::from_value(input)?;
        let store = IntentManifestStore::load()?;

        match input.action.as_str() {
            "discover" => {
                let root = PathBuf::from(required(input.root, "root")?);
                let found = IntentManifestStore::discover(&root, input.max_depth.unwrap_or(6))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&found)?)
                    .with_title(format!("{} intent manifest(s)", found.len())))
            }
            "import" => {
                let path = PathBuf::from(required(input.path, "path")?);
                let manifest = store.import_manifest(&path)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&manifest)?)
                    .with_title(format!("Imported intents for {}", manifest.name)))
            }
            "list" => {
                let actions =
                    store.list_actions(input.query.as_deref(), input.limit.unwrap_or(20))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&actions)?)
                    .with_title(format!("{} intent action(s)", actions.len())))
            }
            "get" => {
                let app_id = required(input.app_id, "app_id")?;
                let manifest = store
                    .get_manifest(&app_id)?
                    .ok_or_else(|| anyhow!("unknown intent app {}", app_id))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&manifest)?)
                    .with_title(format!("Intent app {}", manifest.name)))
            }
            "plan" => {
                let plan = store.plan_action(IntentActionPlanRequest {
                    app_id: required(input.app_id, "app_id")?,
                    action_id: required(input.action_id, "action_id")?,
                    parameters: input.parameters.unwrap_or_else(|| json!({})),
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&plan)?)
                    .with_title(format!("Intent plan: {}", plan.title))
                    .with_metadata(json!({ "intent_plan": plan })))
            }
            other => Err(anyhow!("unsupported intent action '{}'", other)),
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
