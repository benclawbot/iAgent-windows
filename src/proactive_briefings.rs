use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ProactiveBriefingStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProactiveBriefingState {
    #[serde(default)]
    pub recaps: Vec<BriefingCard>,
    #[serde(default)]
    pub feedback: Vec<NeverSuggestRule>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MorningBriefingRequest {
    pub as_of: String,
    #[serde(default)]
    pub focus: Option<String>,
    #[serde(default)]
    pub calendar: Vec<BriefingCalendarItem>,
    #[serde(default)]
    pub due_reminders: Vec<String>,
    #[serde(default)]
    pub projects: Vec<BriefingProjectInput>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BriefingCalendarItem {
    pub title: String,
    pub starts_at: String,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default)]
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BriefingProjectInput {
    pub name: String,
    #[serde(default)]
    pub recent_activity: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EndTaskRecapRequest {
    pub task_title: String,
    #[serde(default)]
    pub completed_steps: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MeetingPrepRequest {
    pub title: String,
    #[serde(default)]
    pub starts_at: Option<String>,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default)]
    pub agenda_hints: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProjectResumeRequest {
    pub project_name: String,
    #[serde(default)]
    pub recent_activity: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RecommendationRequest {
    #[serde(default)]
    pub active_app: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub activity: Option<String>,
    #[serde(default)]
    pub signals: Vec<String>,
    #[serde(default)]
    pub limit: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NeverSuggestRequest {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BriefingCard {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub sections: Vec<BriefingSection>,
    #[serde(default)]
    pub actions: Vec<NextBestAction>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BriefingSection {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NextBestAction {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub rationale: String,
    pub confidence: f32,
    #[serde(default)]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NeverSuggestRule {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ProactiveBriefingStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::jcode_dir()?.join("proactive");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("briefings.json"),
        })
    }

    pub fn state(&self) -> Result<ProactiveBriefingState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read proactive briefings at {}", self.path.display()))
        } else {
            Ok(ProactiveBriefingState::default())
        }
    }

    fn save_state(&self, state: &ProactiveBriefingState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write proactive briefings at {}", self.path.display()))
    }

    pub fn morning_briefing(&self, request: MorningBriefingRequest) -> Result<BriefingCard> {
        if request.as_of.trim().is_empty() {
            return Err(anyhow!("as_of is required"));
        }
        let mut sections = Vec::new();
        if let Some(focus) = request
            .focus
            .as_ref()
            .filter(|focus| !focus.trim().is_empty())
        {
            sections.push(section("Focus", vec![focus.clone()]));
        }
        if !request.calendar.is_empty() {
            sections.push(section(
                "Meetings",
                request
                    .calendar
                    .iter()
                    .map(|item| format!("{} at {}", item.title, item.starts_at))
                    .collect(),
            ));
        }
        if !request.due_reminders.is_empty() {
            sections.push(section("Due reminders", request.due_reminders.clone()));
        }
        if !request.projects.is_empty() {
            sections.push(section(
                "Projects",
                request.projects.iter().map(summarize_project).collect(),
            ));
        }

        let mut actions = Vec::new();
        for item in &request.calendar {
            actions.push(next_action(
                "meeting_prep",
                &format!("Prep for {}", item.title),
                "Upcoming meeting detected in today's briefing.",
                0.86,
                item.source_ref.clone().into_iter().collect(),
            ));
        }
        for reminder in request.due_reminders.iter().take(3) {
            actions.push(next_action(
                "follow_up",
                &format!("Handle reminder: {}", reminder),
                "Reminder is due or called out in the morning briefing.",
                0.82,
                Vec::new(),
            ));
        }
        for project in &request.projects {
            actions.push(next_action(
                "project_resume",
                &format!("Resume {}", project.name),
                "Recent project context is available for a low-friction restart.",
                0.8,
                Vec::new(),
            ));
        }

        let card = BriefingCard {
            id: Uuid::new_v4().to_string(),
            kind: "morning_briefing".to_string(),
            title: "Morning briefing".to_string(),
            summary: format!(
                "Briefing for {} with {} meeting(s), {} reminder(s), and {} project(s).",
                request.as_of,
                request.calendar.len(),
                request.due_reminders.len(),
                request.projects.len()
            ),
            sections,
            actions: self.filter_actions(actions)?,
            source_refs: request
                .calendar
                .iter()
                .filter_map(|item| item.source_ref.clone())
                .collect(),
            evidence_refs: Vec::new(),
            created_at: Utc::now(),
        };
        Ok(card)
    }

    pub fn end_task_recap(&self, request: EndTaskRecapRequest) -> Result<BriefingCard> {
        if request.task_title.trim().is_empty() {
            return Err(anyhow!("task_title is required"));
        }
        let mut sections = vec![section("Completed", request.completed_steps.clone())];
        if !request.evidence_refs.is_empty() {
            sections.push(section("Evidence", request.evidence_refs.clone()));
        }
        if !request.open_questions.is_empty() {
            sections.push(section("Open questions", request.open_questions.clone()));
        }
        if !request.next_actions.is_empty() {
            sections.push(section("Next actions", request.next_actions.clone()));
        }
        let actions = request
            .next_actions
            .iter()
            .map(|action| {
                next_action(
                    "follow_up",
                    action,
                    "Follow-up captured from the end-of-task recap.",
                    0.84,
                    request.evidence_refs.clone(),
                )
            })
            .collect();
        let card = BriefingCard {
            id: Uuid::new_v4().to_string(),
            kind: "end_task_recap".to_string(),
            title: format!("Task recap: {}", request.task_title),
            summary: format!(
                "{} completed step(s), {} evidence reference(s), {} follow-up(s).",
                request.completed_steps.len(),
                request.evidence_refs.len(),
                request.next_actions.len()
            ),
            sections,
            actions: self.filter_actions(actions)?,
            source_refs: Vec::new(),
            evidence_refs: request.evidence_refs,
            created_at: Utc::now(),
        };

        let mut state = self.state()?;
        state.recaps.insert(0, card.clone());
        self.save_state(&state)?;
        Ok(card)
    }

    pub fn meeting_prep(&self, request: MeetingPrepRequest) -> Result<BriefingCard> {
        if request.title.trim().is_empty() {
            return Err(anyhow!("meeting title is required"));
        }
        let mut sections = Vec::new();
        if let Some(starts_at) = request.starts_at.as_ref() {
            sections.push(section("Time", vec![starts_at.clone()]));
        }
        if !request.participants.is_empty() {
            sections.push(section("Participants", request.participants.clone()));
        }
        if !request.agenda_hints.is_empty() {
            sections.push(section("Agenda", request.agenda_hints.clone()));
        }
        if !request.source_refs.is_empty() {
            sections.push(section("Sources", request.source_refs.clone()));
        }

        let actions = self.filter_actions(vec![
            next_action(
                "agenda_review",
                &format!("Review agenda for {}", request.title),
                "Meeting prep should confirm goals, owners, and unresolved questions.",
                0.87,
                request.source_refs.clone(),
            ),
            next_action(
                "draft_followups",
                &format!("Draft follow-ups for {}", request.title),
                "Pre-writing follow-ups makes the meeting outcome easier to capture.",
                0.72,
                request.source_refs.clone(),
            ),
        ])?;
        Ok(BriefingCard {
            id: Uuid::new_v4().to_string(),
            kind: "meeting_prep".to_string(),
            title: format!("Meeting prep: {}", request.title),
            summary: format!(
                "Prepare for {} with {} participant(s) and {} agenda hint(s).",
                request.title,
                request.participants.len(),
                request.agenda_hints.len()
            ),
            sections,
            actions,
            source_refs: request.source_refs,
            evidence_refs: Vec::new(),
            created_at: Utc::now(),
        })
    }

    pub fn project_resume(&self, request: ProjectResumeRequest) -> Result<BriefingCard> {
        if request.project_name.trim().is_empty() {
            return Err(anyhow!("project_name is required"));
        }
        let mut sections = Vec::new();
        if !request.recent_activity.is_empty() {
            sections.push(section("Recent activity", request.recent_activity.clone()));
        }
        if !request.blockers.is_empty() {
            sections.push(section("Blockers", request.blockers.clone()));
        }
        if !request.next_actions.is_empty() {
            sections.push(section("Next actions", request.next_actions.clone()));
        }
        let actions = request
            .next_actions
            .iter()
            .map(|action| {
                next_action(
                    "project_resume",
                    action,
                    "Project resume card surfaced this as the next useful action.",
                    0.83,
                    request.source_refs.clone(),
                )
            })
            .collect();
        Ok(BriefingCard {
            id: Uuid::new_v4().to_string(),
            kind: "project_resume".to_string(),
            title: format!("Resume {}", request.project_name),
            summary: format!(
                "{} recent item(s), {} blocker(s), {} next action(s).",
                request.recent_activity.len(),
                request.blockers.len(),
                request.next_actions.len()
            ),
            sections,
            actions: self.filter_actions(actions)?,
            source_refs: request.source_refs,
            evidence_refs: Vec::new(),
            created_at: Utc::now(),
        })
    }

    pub fn recommend(&self, request: RecommendationRequest) -> Result<Vec<NextBestAction>> {
        let context = [
            request.active_app.as_deref().unwrap_or_default(),
            request.window_title.as_deref().unwrap_or_default(),
            request.activity.as_deref().unwrap_or_default(),
            &request.signals.join(" "),
        ]
        .join(" ")
        .to_ascii_lowercase();
        let mut actions = Vec::new();

        if contains_any(&context, &["meeting", "call", "standup", "review"]) {
            actions.push(next_action(
                "meeting_prep",
                "Prepare for the current meeting",
                "Meeting context is active, so a prep card may be useful.",
                0.86,
                vec!["context:active_window".into()],
            ));
        }
        if contains_any(&context, &["project", "repo", "roadmap", "branch"]) {
            actions.push(next_action(
                "project_resume",
                "Resume the active project",
                "Project context is visible and recent activity can be summarized.",
                0.8,
                vec!["context:active_window".into()],
            ));
        }
        if contains_any(&context, &["done", "completed", "shipped", "finished"]) {
            actions.push(next_action(
                "end_task_recap",
                "Capture an end-of-task recap",
                "The current context looks like a task just finished.",
                0.78,
                vec!["context:activity".into()],
            ));
        }
        if contains_any(&context, &["weekly", "report", "status"]) {
            actions.push(next_action(
                "weekly_report",
                "Draft a status report",
                "Reporting language is present in the current context.",
                0.74,
                vec!["context:activity".into()],
            ));
        }
        if contains_any(&context, &["blocked", "blocker", "stuck", "failing"]) {
            actions.push(next_action(
                "unblock",
                "Create an unblock plan",
                "A blocker signal is present and may need a short action plan.",
                0.82,
                vec!["context:activity".into()],
            ));
        }

        let mut filtered = self.filter_actions(actions)?;
        filtered.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        filtered.truncate(request.limit.max(1));
        Ok(dedupe_by_kind(filtered))
    }

    pub fn never_suggest(&self, request: NeverSuggestRequest) -> Result<NeverSuggestRule> {
        let kind = request.kind.and_then(non_empty);
        let pattern = request.pattern.and_then(non_empty);
        if kind.is_none() && pattern.is_none() {
            return Err(anyhow!("kind or pattern is required"));
        }
        let mut state = self.state()?;
        let rule = NeverSuggestRule {
            id: Uuid::new_v4().to_string(),
            kind,
            pattern,
            reason: request.reason.and_then(non_empty),
            created_at: Utc::now(),
        };
        state.feedback.insert(0, rule.clone());
        self.save_state(&state)?;
        Ok(rule)
    }

    pub fn list_recaps(&self, limit: usize) -> Result<Vec<BriefingCard>> {
        Ok(self
            .state()?
            .recaps
            .into_iter()
            .take(limit.max(1))
            .collect())
    }

    pub fn list_feedback(&self) -> Result<Vec<NeverSuggestRule>> {
        Ok(self.state()?.feedback)
    }

    fn filter_actions(&self, actions: Vec<NextBestAction>) -> Result<Vec<NextBestAction>> {
        let feedback = self.state()?.feedback;
        Ok(actions
            .into_iter()
            .filter(|action| !feedback.iter().any(|rule| rule_matches(rule, action)))
            .collect())
    }
}

fn section(title: &str, items: Vec<String>) -> BriefingSection {
    BriefingSection {
        title: title.to_string(),
        items,
    }
}

fn next_action(
    kind: &str,
    title: &str,
    rationale: &str,
    confidence: f32,
    source_refs: Vec<String>,
) -> NextBestAction {
    NextBestAction {
        id: Uuid::new_v4().to_string(),
        kind: kind.to_string(),
        title: title.to_string(),
        rationale: rationale.to_string(),
        confidence,
        source_refs,
    }
}

fn summarize_project(project: &BriefingProjectInput) -> String {
    let mut parts = vec![project.name.clone()];
    if let Some(activity) = project.recent_activity.first() {
        parts.push(format!("recent: {}", activity));
    }
    if let Some(blocker) = project.blockers.first() {
        parts.push(format!("blocker: {}", blocker));
    }
    if let Some(next) = project.next_actions.first() {
        parts.push(format!("next: {}", next));
    }
    parts.join(" | ")
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn rule_matches(rule: &NeverSuggestRule, action: &NextBestAction) -> bool {
    let kind_matches = rule
        .kind
        .as_ref()
        .map(|kind| kind.eq_ignore_ascii_case(&action.kind))
        .unwrap_or(false);
    let text = format!("{} {}", action.title, action.rationale).to_ascii_lowercase();
    let pattern_matches = rule
        .pattern
        .as_ref()
        .map(|pattern| text.contains(&pattern.to_ascii_lowercase()))
        .unwrap_or(false);
    kind_matches || pattern_matches
}

fn dedupe_by_kind(actions: Vec<NextBestAction>) -> Vec<NextBestAction> {
    let mut seen = HashSet::new();
    actions
        .into_iter()
        .filter(|action| seen.insert(action.kind.clone()))
        .collect()
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
