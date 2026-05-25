use super::{Tool, ToolContext, ToolOutput};
use crate::meeting_memory::{
    MeetingMemoryStore, MeetingSegmentInput, MeetingSessionInput, MeetingTaskConversionRequest,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct MeetingTool;

impl MeetingTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct MeetingInput {
    action: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    participants: Vec<String>,
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    start_ms: Option<u64>,
    #[serde(default)]
    end_ms: Option<u64>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    action_item_ids: Vec<String>,
    #[serde(default)]
    create_reminders: Option<bool>,
    #[serde(default)]
    create_jobs: Option<bool>,
    #[serde(default)]
    create_delegation_drafts: Option<bool>,
}

#[async_trait]
impl Tool for MeetingTool {
    fn name(&self) -> &str {
        "meeting"
    }

    fn description(&self) -> &str {
        "Capture meeting transcript segments, finish source-linked meeting notes, and convert action items into reminders, jobs, or delegation drafts."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["start", "append_segment", "finish", "get", "list", "convert_action_items"],
                    "description": "Meeting memory action."
                },
                "session_id": {"type": "string"},
                "title": {"type": "string"},
                "participants": {"type": "array", "items": {"type": "string"}},
                "speaker": {"type": "string"},
                "start_ms": {"type": "integer"},
                "end_ms": {"type": "integer"},
                "text": {"type": "string"},
                "limit": {"type": "integer"},
                "action_item_ids": {"type": "array", "items": {"type": "string"}},
                "create_reminders": {"type": "boolean"},
                "create_jobs": {"type": "boolean"},
                "create_delegation_drafts": {"type": "boolean"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: MeetingInput = serde_json::from_value(input)?;
        let store = MeetingMemoryStore::load()?;

        match input.action.as_str() {
            "start" => {
                let session = store.start_session(MeetingSessionInput {
                    title: required(input.title, "title")?,
                    participants: input.participants,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&session)?)
                    .with_title(format!("Meeting started: {}", session.title)))
            }
            "append_segment" => {
                let segment = store.append_segment(MeetingSegmentInput {
                    session_id: required(input.session_id, "session_id")?,
                    speaker: input.speaker,
                    start_ms: input.start_ms.unwrap_or(0),
                    end_ms: input.end_ms.unwrap_or(0),
                    text: required(input.text, "text")?,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&segment)?)
                    .with_title(format!("Captured meeting segment {}", segment.id)))
            }
            "finish" => {
                let session = store.finish_session(&required(input.session_id, "session_id")?)?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&session)?)
                    .with_title(format!("Meeting finished: {}", session.title)))
            }
            "get" => {
                let session = store
                    .get_session(&required(input.session_id, "session_id")?)?
                    .ok_or_else(|| anyhow!("meeting session not found"))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&session)?)
                    .with_title(format!("Meeting: {}", session.title)))
            }
            "list" => {
                let sessions = store.list_sessions(input.limit.unwrap_or(10))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&sessions)?)
                    .with_title(format!("{} meeting sessions", sessions.len())))
            }
            "convert_action_items" => {
                let conversion = store.convert_action_items(MeetingTaskConversionRequest {
                    session_id: required(input.session_id, "session_id")?,
                    action_item_ids: input.action_item_ids,
                    create_reminders: input.create_reminders.unwrap_or(true),
                    create_jobs: input.create_jobs.unwrap_or(false),
                    create_delegation_drafts: input.create_delegation_drafts.unwrap_or(false),
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&conversion)?)
                    .with_title("Meeting action items converted".to_string())
                    .with_metadata(json!({ "meeting_conversion": conversion })))
            }
            other => Err(anyhow!("unsupported meeting action '{}'", other)),
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
