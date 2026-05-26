use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProcessingReportStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingReportState {
    #[serde(default)]
    pub records: Vec<ProcessingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingRecordInput {
    pub purpose: String,
    pub processor: String,
    pub environment: String,
    #[serde(default)]
    pub data_categories: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    pub retention: String,
    pub user_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingRecord {
    pub id: String,
    pub purpose: String,
    pub processor: String,
    pub environment: String,
    #[serde(default)]
    pub data_categories: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    pub retention: String,
    pub user_visible: bool,
    pub recorded_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingReportQuery {
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub processor: Option<String>,
    #[serde(default)]
    pub data_category: Option<String>,
    #[serde(default)]
    pub include_deleted: bool,
    #[serde(default)]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingReport {
    pub total_records: usize,
    #[serde(default)]
    pub by_environment: BTreeMap<String, usize>,
    #[serde(default)]
    pub by_processor: BTreeMap<String, usize>,
    #[serde(default)]
    pub by_data_category: BTreeMap<String, usize>,
    #[serde(default)]
    pub records: Vec<ProcessingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessingDeletionRequest {
    pub record_id: String,
    pub reason: String,
}

impl ProcessingReportStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::jcode_dir()?.join("processing_report");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("records.json"),
        })
    }

    pub fn state(&self) -> Result<ProcessingReportState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read processing report at {}", self.path.display()))
        } else {
            Ok(ProcessingReportState::default())
        }
    }

    fn save_state(&self, state: &ProcessingReportState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write processing report at {}", self.path.display()))
    }

    pub fn record(&self, input: ProcessingRecordInput) -> Result<ProcessingRecord> {
        require_text("purpose", &input.purpose)?;
        require_text("processor", &input.processor)?;
        require_text("environment", &input.environment)?;
        require_text("retention", &input.retention)?;
        let record = ProcessingRecord {
            id: Uuid::new_v4().to_string(),
            purpose: input.purpose,
            processor: input.processor,
            environment: input.environment,
            data_categories: normalize_list(input.data_categories),
            source_refs: normalize_list(input.source_refs),
            retention: input.retention,
            user_visible: input.user_visible,
            recorded_at: Utc::now(),
            deleted_at: None,
            deleted_reason: None,
        };
        let mut state = self.state()?;
        state.records.insert(0, record.clone());
        self.save_state(&state)?;
        Ok(record)
    }

    pub fn report(&self, query: ProcessingReportQuery) -> Result<ProcessingReport> {
        let mut records: Vec<_> = self
            .state()?
            .records
            .into_iter()
            .filter(|record| query.include_deleted || record.deleted_at.is_none())
            .filter(|record| {
                query
                    .environment
                    .as_ref()
                    .map(|environment| record.environment.eq_ignore_ascii_case(environment))
                    .unwrap_or(true)
            })
            .filter(|record| {
                query
                    .processor
                    .as_ref()
                    .map(|processor| {
                        record
                            .processor
                            .to_lowercase()
                            .contains(&processor.to_lowercase())
                    })
                    .unwrap_or(true)
            })
            .filter(|record| {
                query
                    .data_category
                    .as_ref()
                    .map(|category| {
                        record
                            .data_categories
                            .iter()
                            .any(|value| value.eq_ignore_ascii_case(category))
                    })
                    .unwrap_or(true)
            })
            .collect();
        records.truncate(query.limit.max(1));
        Ok(build_report(records))
    }

    pub fn history(&self, limit: usize) -> Result<Vec<ProcessingRecord>> {
        Ok(self
            .state()?
            .records
            .into_iter()
            .take(limit.max(1))
            .collect())
    }

    pub fn export_markdown(&self, query: ProcessingReportQuery) -> Result<String> {
        let report = self.report(query)?;
        let mut out = String::new();
        out.push_str("# iAgent Processing Transparency Report\n\n");
        out.push_str(&format!("Total records: {}\n\n", report.total_records));
        out.push_str("## Environments\n\n");
        for (environment, count) in &report.by_environment {
            out.push_str(&format!("- {}: {}\n", environment, count));
        }
        out.push_str("\n## Processors\n\n");
        for (processor, count) in &report.by_processor {
            out.push_str(&format!("- {}: {}\n", processor, count));
        }
        out.push_str("\n## Records\n\n");
        for record in &report.records {
            out.push_str(&format!(
                "- {} | {} | {} | {} | retention: {}\n",
                record.recorded_at.to_rfc3339(),
                record.environment,
                record.processor,
                record.purpose,
                record.retention
            ));
            if !record.data_categories.is_empty() {
                out.push_str(&format!(
                    "  - data: {}\n",
                    record.data_categories.join(", ")
                ));
            }
            if !record.source_refs.is_empty() {
                out.push_str(&format!("  - sources: {}\n", record.source_refs.join(", ")));
            }
            if let Some(reason) = &record.deleted_reason {
                out.push_str(&format!("  - deleted: {}\n", reason));
            }
        }
        Ok(out)
    }

    pub fn mark_deleted(&self, request: ProcessingDeletionRequest) -> Result<ProcessingRecord> {
        require_text("record_id", &request.record_id)?;
        require_text("reason", &request.reason)?;
        let mut state = self.state()?;
        let Some(record) = state
            .records
            .iter_mut()
            .find(|record| record.id == request.record_id)
        else {
            return Err(anyhow!("unknown processing record {}", request.record_id));
        };
        record.deleted_at = Some(Utc::now());
        record.deleted_reason = Some(request.reason);
        let updated = record.clone();
        self.save_state(&state)?;
        Ok(updated)
    }
}

fn build_report(records: Vec<ProcessingRecord>) -> ProcessingReport {
    let mut by_environment = BTreeMap::new();
    let mut by_processor = BTreeMap::new();
    let mut by_data_category = BTreeMap::new();
    for record in &records {
        *by_environment
            .entry(record.environment.clone())
            .or_insert(0) += 1;
        *by_processor.entry(record.processor.clone()).or_insert(0) += 1;
        for category in &record.data_categories {
            *by_data_category.entry(category.clone()).or_insert(0) += 1;
        }
    }
    ProcessingReport {
        total_records: records.len(),
        by_environment,
        by_processor,
        by_data_category,
        records,
    }
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
    let mut values: Vec<_> = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
}

fn require_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{label} is required"));
    }
    Ok(())
}
