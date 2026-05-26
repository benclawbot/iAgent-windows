use super::{Tool, ToolContext, ToolOutput};
use crate::processing_report::{
    ProcessingDeletionRequest, ProcessingRecordInput, ProcessingReportQuery, ProcessingReportStore,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct ProcessingReportTool;

impl ProcessingReportTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct ProcessingReportInput {
    action: String,
    #[serde(default)]
    record_id: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    processor: Option<String>,
    #[serde(default)]
    environment: Option<String>,
    #[serde(default)]
    data_categories: Vec<String>,
    #[serde(default)]
    source_refs: Vec<String>,
    #[serde(default)]
    retention: Option<String>,
    #[serde(default)]
    user_visible: Option<bool>,
    #[serde(default)]
    data_category: Option<String>,
    #[serde(default)]
    include_deleted: Option<bool>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for ProcessingReportTool {
    fn name(&self) -> &str {
        "processing_report"
    }

    fn description(&self) -> &str {
        "Record and inspect where iAgent processed user data: local device, private cloud, external model, connector, retention, deletion state, and exportable privacy reports."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["record", "report", "export_markdown", "mark_deleted", "history"],
                    "description": "Processing report action."
                },
                "record_id": {"type": "string"},
                "purpose": {"type": "string"},
                "processor": {"type": "string"},
                "environment": {"type": "string"},
                "data_categories": {"type": "array", "items": {"type": "string"}},
                "source_refs": {"type": "array", "items": {"type": "string"}},
                "retention": {"type": "string"},
                "user_visible": {"type": "boolean"},
                "data_category": {"type": "string"},
                "include_deleted": {"type": "boolean"},
                "reason": {"type": "string"},
                "limit": {"type": "integer"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: ProcessingReportInput = serde_json::from_value(input)?;
        let store = ProcessingReportStore::load()?;

        match input.action.as_str() {
            "record" => {
                let record = store.record(ProcessingRecordInput {
                    purpose: required(input.purpose, "purpose")?,
                    processor: required(input.processor, "processor")?,
                    environment: required(input.environment, "environment")?,
                    data_categories: input.data_categories,
                    source_refs: input.source_refs,
                    retention: required(input.retention, "retention")?,
                    user_visible: input.user_visible.unwrap_or(true),
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&record)?)
                    .with_title(format!("Processing record {}", record.id)))
            }
            "report" => {
                let report = store.report(query_from_input(&input))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&report)?)
                    .with_title(format!("{} processing record(s)", report.total_records))
                    .with_metadata(json!({ "processing_report": report })))
            }
            "export_markdown" => {
                let markdown = store.export_markdown(query_from_input(&input))?;
                Ok(ToolOutput::new(markdown)
                    .with_title("Processing transparency report".to_string()))
            }
            "mark_deleted" => {
                let record = store.mark_deleted(ProcessingDeletionRequest {
                    record_id: required(input.record_id, "record_id")?,
                    reason: required(input.reason, "reason")?,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&record)?)
                    .with_title(format!("Processing record deleted {}", record.id)))
            }
            "history" => {
                let history = store.history(input.limit.unwrap_or(20))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&history)?)
                    .with_title(format!("{} processing record(s)", history.len())))
            }
            other => Err(anyhow!("unsupported processing_report action '{}'", other)),
        }
    }
}

fn query_from_input(input: &ProcessingReportInput) -> ProcessingReportQuery {
    ProcessingReportQuery {
        environment: input.environment.clone(),
        processor: input.processor.clone(),
        data_category: input.data_category.clone(),
        include_deleted: input.include_deleted.unwrap_or(false),
        limit: input.limit.unwrap_or(50),
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
