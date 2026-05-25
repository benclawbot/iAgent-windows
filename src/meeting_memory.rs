use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::personal_layer::{JobInput, PersonalStore, ReminderInput};

#[derive(Debug, Clone)]
pub struct MeetingMemoryStore {
    path: std::path::PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MeetingMemoryState {
    #[serde(default)]
    pub sessions: Vec<MeetingSession>,
}

#[derive(Debug, Clone)]
pub struct MeetingSessionInput {
    pub title: String,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MeetingSegmentInput {
    pub session_id: String,
    pub speaker: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeetingSession {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub participants: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: String,
    #[serde(default)]
    pub segments: Vec<MeetingSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<MeetingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingSegment {
    pub id: String,
    pub speaker: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingSummary {
    pub overview: String,
    pub decisions: Vec<MeetingDecision>,
    pub action_items: Vec<MeetingActionItem>,
    pub questions: Vec<MeetingQuestion>,
    pub source_notes: Vec<SourceLinkedNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingDecision {
    pub text: String,
    pub source_segment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingActionItem {
    pub id: String,
    pub text: String,
    pub owner: Option<String>,
    pub due_hint: Option<String>,
    pub source_segment_id: String,
    #[serde(default)]
    pub converted_to: Vec<MeetingConversionTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingQuestion {
    pub text: String,
    pub source_segment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceLinkedNote {
    pub text: String,
    pub source_segment_id: String,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingConversionTarget {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeetingDelegationDraft {
    pub action_item_id: String,
    pub tool: String,
    pub instruction: String,
    pub source_segment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeetingTaskConversion {
    pub reminders: Vec<crate::personal_layer::Reminder>,
    pub jobs: Vec<crate::personal_layer::BackgroundJob>,
    pub delegation_drafts: Vec<MeetingDelegationDraft>,
}

#[derive(Debug, Clone)]
pub struct MeetingTaskConversionRequest {
    pub session_id: String,
    pub action_item_ids: Vec<String>,
    pub create_reminders: bool,
    pub create_jobs: bool,
    pub create_delegation_drafts: bool,
}

impl MeetingMemoryStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::jcode_dir()?.join("meetings");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("meeting_memory.json"),
        })
    }

    pub fn state(&self) -> Result<MeetingMemoryState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
        } else {
            Ok(MeetingMemoryState::default())
        }
    }

    fn save_state(&self, state: &MeetingMemoryState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
    }

    pub fn start_session(&self, input: MeetingSessionInput) -> Result<MeetingSession> {
        if input.title.trim().is_empty() {
            return Err(anyhow!("meeting title is required"));
        }
        let mut state = self.state()?;
        let session = MeetingSession {
            id: Uuid::new_v4().to_string(),
            title: input.title,
            participants: input.participants,
            started_at: Utc::now(),
            ended_at: None,
            status: "active".to_string(),
            segments: Vec::new(),
            summary: None,
        };
        state.sessions.insert(0, session.clone());
        self.save_state(&state)?;
        Ok(session)
    }

    pub fn append_segment(&self, input: MeetingSegmentInput) -> Result<MeetingSegment> {
        if input.text.trim().is_empty() {
            return Err(anyhow!("meeting segment text is required"));
        }
        let mut state = self.state()?;
        let session = state
            .sessions
            .iter_mut()
            .find(|session| session.id == input.session_id)
            .ok_or_else(|| anyhow!("unknown meeting session {}", input.session_id))?;
        if session.status != "active" {
            return Err(anyhow!("meeting session is not active"));
        }
        let segment = MeetingSegment {
            id: format!("seg-{:03}", session.segments.len() + 1),
            speaker: input.speaker,
            start_ms: input.start_ms,
            end_ms: input.end_ms,
            text: input.text,
            added_at: Utc::now(),
        };
        session.segments.push(segment.clone());
        self.save_state(&state)?;
        Ok(segment)
    }

    pub fn finish_session(&self, session_id: &str) -> Result<MeetingSession> {
        let mut state = self.state()?;
        let session = state
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
            .ok_or_else(|| anyhow!("unknown meeting session {}", session_id))?;
        let summary = summarize_session(session);
        session.summary = Some(summary);
        session.status = "complete".to_string();
        session.ended_at = Some(Utc::now());
        let finished = session.clone();
        self.save_state(&state)?;
        Ok(finished)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<MeetingSession>> {
        Ok(self
            .state()?
            .sessions
            .into_iter()
            .find(|session| session.id == session_id))
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<MeetingSession>> {
        Ok(self
            .state()?
            .sessions
            .into_iter()
            .take(limit.max(1))
            .collect())
    }

    pub fn convert_action_items(
        &self,
        request: MeetingTaskConversionRequest,
    ) -> Result<MeetingTaskConversion> {
        let mut state = self.state()?;
        let session = state
            .sessions
            .iter_mut()
            .find(|session| session.id == request.session_id)
            .ok_or_else(|| anyhow!("unknown meeting session {}", request.session_id))?;
        let summary = session
            .summary
            .as_mut()
            .ok_or_else(|| anyhow!("meeting session must be finished before conversion"))?;
        let personal = PersonalStore::load()?;
        let mut reminders = Vec::new();
        let mut jobs = Vec::new();
        let mut delegation_drafts = Vec::new();
        let tomorrow = (Utc::now() + chrono::Duration::days(1)).to_rfc3339();

        for item in &mut summary.action_items {
            if !request.action_item_ids.is_empty() && !request.action_item_ids.contains(&item.id) {
                continue;
            }
            if request.create_reminders {
                let reminder = personal.create_reminder(ReminderInput {
                    title: item.text.clone(),
                    note: Some(format!(
                        "From meeting '{}' segment {}",
                        session.title, item.source_segment_id
                    )),
                    due_at: tomorrow.clone(),
                    source_app: Some("iAgent Meeting Memory".to_string()),
                    source_title: Some(session.title.clone()),
                })?;
                item.converted_to.push(MeetingConversionTarget {
                    kind: "reminder".to_string(),
                    id: reminder.id.clone(),
                });
                reminders.push(reminder);
            }
            if request.create_jobs {
                let job = personal.create_job(JobInput {
                    kind: "meeting_action_item".to_string(),
                    description: item.text.clone(),
                    input_json: json!({
                        "meeting_id": session.id,
                        "meeting_title": session.title,
                        "action_item_id": item.id,
                        "source_segment_id": item.source_segment_id,
                    }),
                })?;
                item.converted_to.push(MeetingConversionTarget {
                    kind: "job".to_string(),
                    id: job.id.clone(),
                });
                jobs.push(job);
            }
            if request.create_delegation_drafts {
                let draft = MeetingDelegationDraft {
                    action_item_id: item.id.clone(),
                    tool: "communicate".to_string(),
                    instruction: format!(
                        "Follow up on meeting action item from '{}': {}",
                        session.title, item.text
                    ),
                    source_segment_id: item.source_segment_id.clone(),
                };
                item.converted_to.push(MeetingConversionTarget {
                    kind: "delegation_draft".to_string(),
                    id: draft.action_item_id.clone(),
                });
                delegation_drafts.push(draft);
            }
        }

        self.save_state(&state)?;
        Ok(MeetingTaskConversion {
            reminders,
            jobs,
            delegation_drafts,
        })
    }
}

fn summarize_session(session: &MeetingSession) -> MeetingSummary {
    let mut decisions = Vec::new();
    let mut action_items = Vec::new();
    let mut questions = Vec::new();
    let mut source_notes = Vec::new();

    for segment in &session.segments {
        if let Some(text) = extract_after_prefix(&segment.text, &["decision:", "decided:"]) {
            decisions.push(MeetingDecision {
                text: text.clone(),
                source_segment_id: segment.id.clone(),
            });
            source_notes.push(source_note("Decision", &text, segment));
        } else if segment.text.to_lowercase().contains("we decided") {
            let text = segment.text.trim().to_string();
            decisions.push(MeetingDecision {
                text: text.clone(),
                source_segment_id: segment.id.clone(),
            });
            source_notes.push(source_note("Decision", &text, segment));
        }

        if let Some(text) =
            extract_after_prefix(&segment.text, &["action:", "action item:", "todo:"])
        {
            let item = MeetingActionItem {
                id: format!("act-{}", action_items.len() + 1),
                text: text.clone(),
                owner: segment.speaker.clone(),
                due_hint: extract_due_hint(&text),
                source_segment_id: segment.id.clone(),
                converted_to: Vec::new(),
            };
            source_notes.push(source_note("Action item", &text, segment));
            action_items.push(item);
        }

        if let Some(text) = extract_after_prefix(&segment.text, &["question:"]) {
            questions.push(MeetingQuestion {
                text: text.clone(),
                source_segment_id: segment.id.clone(),
            });
            source_notes.push(source_note("Question", &text, segment));
        } else if segment.text.trim_end().ends_with('?') {
            let text = segment.text.trim().to_string();
            questions.push(MeetingQuestion {
                text: text.clone(),
                source_segment_id: segment.id.clone(),
            });
            source_notes.push(source_note("Question", &text, segment));
        }
    }

    MeetingSummary {
        overview: format!(
            "{} captured {} transcript segment(s), {} decision(s), {} action item(s), and {} question(s).",
            session.title,
            session.segments.len(),
            decisions.len(),
            action_items.len(),
            questions.len()
        ),
        decisions,
        action_items,
        questions,
        source_notes,
    }
}

fn extract_after_prefix(value: &str, prefixes: &[&str]) -> Option<String> {
    let trimmed = value.trim();
    let lower = trimmed.to_lowercase();
    for prefix in prefixes {
        if lower.starts_with(prefix) {
            return Some(trimmed[prefix.len()..].trim().to_string());
        }
    }
    None
}

fn extract_due_hint(value: &str) -> Option<String> {
    let lower = value.to_lowercase();
    ["today", "tomorrow", "next week"]
        .iter()
        .find(|hint| lower.contains(**hint))
        .map(|hint| hint.to_string())
}

fn source_note(kind: &str, text: &str, segment: &MeetingSegment) -> SourceLinkedNote {
    SourceLinkedNote {
        text: format!("{kind}: {text}"),
        source_segment_id: segment.id.clone(),
        quote: segment.text.clone(),
    }
}
