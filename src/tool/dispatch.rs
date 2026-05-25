use super::{Tool, ToolContext, ToolOutput};
use crate::remote_dispatch::{
    DispatchCompletionRequest, DispatchFailureRequest, DispatchSubmitRequest, RemoteDispatchStore,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct DispatchTool;

impl DispatchTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct DispatchInput {
    action: String,
    #[serde(default)]
    client_name: Option<String>,
    #[serde(default)]
    client_token: Option<String>,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    instruction: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    scheduled_for: Option<String>,
    #[serde(default)]
    approval_level: Option<String>,
    #[serde(default)]
    context: Option<Value>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    retry_hint: Option<String>,
    #[serde(default)]
    log_refs: Vec<String>,
    #[serde(default)]
    as_of: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for DispatchTool {
    fn name(&self) -> &str {
        "dispatch"
    }

    fn description(&self) -> &str {
        "Manage authenticated remote/local dispatch: clients, scheduled tasks, approval-needed notifications, mobile status, completion evidence, failure packets, and watch events."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["create_client", "revoke_client", "submit", "approve", "complete", "fail", "status", "list", "watch", "due"],
                    "description": "Remote dispatch action."
                },
                "client_name": {"type": "string"},
                "client_token": {"type": "string"},
                "client_id": {"type": "string"},
                "task_id": {"type": "string"},
                "title": {"type": "string"},
                "instruction": {"type": "string"},
                "origin": {"type": "string"},
                "target": {"type": "string"},
                "scheduled_for": {"type": "string"},
                "approval_level": {"type": "string"},
                "context": {"type": "object", "additionalProperties": true},
                "summary": {"type": "string"},
                "evidence_refs": {"type": "array", "items": {"type": "string"}},
                "error": {"type": "string"},
                "retry_hint": {"type": "string"},
                "log_refs": {"type": "array", "items": {"type": "string"}},
                "as_of": {"type": "string"},
                "limit": {"type": "integer"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: DispatchInput = serde_json::from_value(input)?;
        let store = RemoteDispatchStore::load()?;

        match input.action.as_str() {
            "create_client" => {
                let client = store.create_client(&required(input.client_name, "client_name")?)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&client)?)
                    .with_title(format!("Dispatch client {}", client.client.name)))
            }
            "revoke_client" => {
                let client_id = required(input.client_id, "client_id")?;
                let revoked = store.revoke_client(&client_id)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&json!({
                    "client_id": client_id,
                    "revoked": revoked
                }))?)
                .with_title("Dispatch client revoked".to_string()))
            }
            "submit" => {
                let task = store.submit_task(DispatchSubmitRequest {
                    client_token: required(input.client_token, "client_token")?,
                    title: required(input.title, "title")?,
                    instruction: required(input.instruction, "instruction")?,
                    origin: input.origin.unwrap_or_else(|| "local".to_string()),
                    target: input.target.unwrap_or_else(|| "local".to_string()),
                    scheduled_for: input.scheduled_for,
                    approval_level: input.approval_level,
                    context: input.context.unwrap_or_else(|| json!({})),
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&task)?)
                    .with_title(format!("Dispatch task {}", task.status))
                    .with_metadata(json!({ "dispatch_task": task })))
            }
            "approve" => {
                let task = store.approve_task(&required(input.task_id, "task_id")?)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&task)?)
                    .with_title("Dispatch task approved".to_string()))
            }
            "complete" => {
                let task = store.complete_task(DispatchCompletionRequest {
                    task_id: required(input.task_id, "task_id")?,
                    summary: required(input.summary, "summary")?,
                    evidence_refs: input.evidence_refs,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&task)?)
                    .with_title("Dispatch task completed".to_string()))
            }
            "fail" => {
                let task = store.fail_task(DispatchFailureRequest {
                    task_id: required(input.task_id, "task_id")?,
                    error: required(input.error, "error")?,
                    retry_hint: input.retry_hint,
                    log_refs: input.log_refs,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&task)?)
                    .with_title("Dispatch task failed".to_string()))
            }
            "status" => {
                let status = store
                    .status(&required(input.task_id, "task_id")?)?
                    .ok_or_else(|| anyhow!("dispatch task not found"))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&status)?)
                    .with_title(format!("Dispatch status {}", status.status)))
            }
            "list" => {
                let tasks = store.list_tasks(input.limit.unwrap_or(20))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&tasks)?)
                    .with_title(format!("{} dispatch task(s)", tasks.len())))
            }
            "watch" => {
                let events =
                    store.watch_events(input.task_id.as_deref(), input.limit.unwrap_or(20))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&events)?)
                    .with_title(format!("{} dispatch event(s)", events.len())))
            }
            "due" => {
                let tasks = store.due_tasks(&required(input.as_of, "as_of")?)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&tasks)?)
                    .with_title(format!("{} due dispatch task(s)", tasks.len())))
            }
            other => Err(anyhow!("unsupported dispatch action '{}'", other)),
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
