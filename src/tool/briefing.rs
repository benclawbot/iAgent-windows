use super::{Tool, ToolContext, ToolOutput};
use crate::proactive_briefings::{
    BriefingCalendarItem, BriefingProjectInput, EndTaskRecapRequest, MeetingPrepRequest,
    MorningBriefingRequest, NeverSuggestRequest, ProactiveBriefingStore, ProjectResumeRequest,
    RecommendationRequest,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct BriefingTool;

impl BriefingTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct BriefingInput {
    action: String,
    #[serde(default)]
    as_of: Option<String>,
    #[serde(default)]
    focus: Option<String>,
    #[serde(default)]
    calendar: Vec<BriefingCalendarItem>,
    #[serde(default)]
    due_reminders: Vec<String>,
    #[serde(default)]
    projects: Vec<BriefingProjectInput>,
    #[serde(default)]
    task_title: Option<String>,
    #[serde(default)]
    completed_steps: Vec<String>,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    next_actions: Vec<String>,
    #[serde(default)]
    open_questions: Vec<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    starts_at: Option<String>,
    #[serde(default)]
    participants: Vec<String>,
    #[serde(default)]
    agenda_hints: Vec<String>,
    #[serde(default)]
    source_refs: Vec<String>,
    #[serde(default)]
    project_name: Option<String>,
    #[serde(default)]
    recent_activity: Vec<String>,
    #[serde(default)]
    blockers: Vec<String>,
    #[serde(default)]
    active_app: Option<String>,
    #[serde(default)]
    window_title: Option<String>,
    #[serde(default)]
    activity: Option<String>,
    #[serde(default)]
    signals: Vec<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for BriefingTool {
    fn name(&self) -> &str {
        "briefing"
    }

    fn description(&self) -> &str {
        "Create proactive briefings, meeting prep cards, project resume cards, end-of-task recaps, low-noise next-best actions, and never-suggest feedback rules."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["morning", "end_task_recap", "meeting_prep", "project_resume", "recommend", "never_suggest", "list_recaps", "list_feedback"],
                    "description": "Proactive briefing action."
                },
                "as_of": {"type": "string"},
                "focus": {"type": "string"},
                "calendar": {"type": "array", "items": {"type": "object"}},
                "due_reminders": {"type": "array", "items": {"type": "string"}},
                "projects": {"type": "array", "items": {"type": "object"}},
                "task_title": {"type": "string"},
                "completed_steps": {"type": "array", "items": {"type": "string"}},
                "evidence_refs": {"type": "array", "items": {"type": "string"}},
                "next_actions": {"type": "array", "items": {"type": "string"}},
                "open_questions": {"type": "array", "items": {"type": "string"}},
                "title": {"type": "string"},
                "starts_at": {"type": "string"},
                "participants": {"type": "array", "items": {"type": "string"}},
                "agenda_hints": {"type": "array", "items": {"type": "string"}},
                "source_refs": {"type": "array", "items": {"type": "string"}},
                "project_name": {"type": "string"},
                "recent_activity": {"type": "array", "items": {"type": "string"}},
                "blockers": {"type": "array", "items": {"type": "string"}},
                "active_app": {"type": "string"},
                "window_title": {"type": "string"},
                "activity": {"type": "string"},
                "signals": {"type": "array", "items": {"type": "string"}},
                "kind": {"type": "string"},
                "pattern": {"type": "string"},
                "reason": {"type": "string"},
                "limit": {"type": "integer"}
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: BriefingInput = serde_json::from_value(input)?;
        let store = ProactiveBriefingStore::load()?;

        match input.action.as_str() {
            "morning" => {
                let card = store.morning_briefing(MorningBriefingRequest {
                    as_of: required(input.as_of, "as_of")?,
                    focus: input.focus,
                    calendar: input.calendar,
                    due_reminders: input.due_reminders,
                    projects: input.projects,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&card)?)
                    .with_title(card.title.clone())
                    .with_metadata(json!({ "briefing": card })))
            }
            "end_task_recap" => {
                let recap = store.end_task_recap(EndTaskRecapRequest {
                    task_title: required(input.task_title, "task_title")?,
                    completed_steps: input.completed_steps,
                    evidence_refs: input.evidence_refs,
                    next_actions: input.next_actions,
                    open_questions: input.open_questions,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&recap)?)
                    .with_title(recap.title.clone())
                    .with_metadata(json!({ "briefing_recap": recap })))
            }
            "meeting_prep" => {
                let card = store.meeting_prep(MeetingPrepRequest {
                    title: required(input.title, "title")?,
                    starts_at: input.starts_at,
                    participants: input.participants,
                    agenda_hints: input.agenda_hints,
                    source_refs: input.source_refs,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&card)?)
                    .with_title(card.title.clone())
                    .with_metadata(json!({ "meeting_prep": card })))
            }
            "project_resume" => {
                let card = store.project_resume(ProjectResumeRequest {
                    project_name: required(input.project_name, "project_name")?,
                    recent_activity: input.recent_activity,
                    blockers: input.blockers,
                    next_actions: input.next_actions,
                    source_refs: input.source_refs,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&card)?)
                    .with_title(card.title.clone())
                    .with_metadata(json!({ "project_resume": card })))
            }
            "recommend" => {
                let actions = store.recommend(RecommendationRequest {
                    active_app: input.active_app,
                    window_title: input.window_title,
                    activity: input.activity,
                    signals: input.signals,
                    limit: input.limit.unwrap_or(5),
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&actions)?)
                    .with_title(format!("{} next-best action(s)", actions.len()))
                    .with_metadata(json!({ "next_best_actions": actions })))
            }
            "never_suggest" => {
                let rule = store.never_suggest(NeverSuggestRequest {
                    kind: input.kind,
                    pattern: input.pattern,
                    reason: input.reason,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&rule)?)
                    .with_title(format!("Never suggest rule {}", rule.id)))
            }
            "list_recaps" => {
                let recaps = store.list_recaps(input.limit.unwrap_or(10))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&recaps)?)
                    .with_title(format!("{} task recap(s)", recaps.len())))
            }
            "list_feedback" => {
                let feedback = store.list_feedback()?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&feedback)?)
                    .with_title(format!("{} never-suggest rule(s)", feedback.len())))
            }
            other => Err(anyhow!("unsupported briefing action '{}'", other)),
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{name} is required"))
}
