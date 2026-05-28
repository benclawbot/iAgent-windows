use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RemoteDispatchStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RemoteDispatchState {
    #[serde(default)]
    pub clients: Vec<DispatchClient>,
    #[serde(default)]
    pub tasks: Vec<DispatchTask>,
    #[serde(default)]
    pub events: Vec<DispatchEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchClient {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchClientCreated {
    pub client: DispatchClient,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchSubmitRequest {
    pub client_token: String,
    pub title: String,
    pub instruction: String,
    pub origin: String,
    pub target: String,
    #[serde(default)]
    pub scheduled_for: Option<String>,
    #[serde(default)]
    pub approval_level: Option<String>,
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchTask {
    pub id: String,
    pub title: String,
    pub instruction: String,
    pub origin: String,
    pub target: String,
    pub client_id: String,
    pub status: String,
    pub approval_level: String,
    pub approval_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_for: Option<DateTime<Utc>>,
    #[serde(default)]
    pub context: Value,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_summary: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_packet: Option<DispatchFailurePacket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchCompletionRequest {
    pub task_id: String,
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchFailureRequest {
    pub task_id: String,
    pub error: String,
    #[serde(default)]
    pub retry_hint: Option<String>,
    #[serde(default)]
    pub log_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchFailurePacket {
    pub error: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_hint: Option<String>,
    #[serde(default)]
    pub log_refs: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchMobileStatus {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub approval_required: bool,
    pub approval_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_for: Option<DateTime<Utc>>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_packet: Option<DispatchFailurePacket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchEvent {
    pub id: String,
    pub task_id: String,
    pub kind: String,
    pub message: String,
    pub notify_user: bool,
    pub created_at: DateTime<Utc>,
}

impl RemoteDispatchStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::iagent_dir()?.join("remote_dispatch");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("dispatch.json"),
        })
    }

    pub fn state(&self) -> Result<RemoteDispatchState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read remote dispatch at {}", self.path.display()))
        } else {
            Ok(RemoteDispatchState::default())
        }
    }

    fn save_state(&self, state: &RemoteDispatchState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write remote dispatch at {}", self.path.display()))
    }

    pub fn create_client(&self, name: &str) -> Result<DispatchClientCreated> {
        if name.trim().is_empty() {
            return Err(anyhow!("client name is required"));
        }
        let token = format!("rdt_{}", Uuid::new_v4().simple());
        let client = DispatchClient {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            token_hash: hash_token(&token),
            created_at: Utc::now(),
            revoked_at: None,
        };
        let mut state = self.state()?;
        state.clients.insert(0, client.clone());
        self.save_state(&state)?;
        Ok(DispatchClientCreated { client, token })
    }

    pub fn revoke_client(&self, client_id: &str) -> Result<bool> {
        let mut state = self.state()?;
        let mut found = false;
        for client in &mut state.clients {
            if client.id == client_id && client.revoked_at.is_none() {
                client.revoked_at = Some(Utc::now());
                found = true;
            }
        }
        if found {
            self.save_state(&state)?;
        }
        Ok(found)
    }

    pub fn submit_task(&self, request: DispatchSubmitRequest) -> Result<DispatchTask> {
        let mut state = self.state()?;
        let client = authenticate(&state, &request.client_token)?;
        require_text("title", &request.title)?;
        require_text("instruction", &request.instruction)?;
        require_text("origin", &request.origin)?;
        require_text("target", &request.target)?;

        let scheduled_for = parse_optional_time(request.scheduled_for)?;
        let approval_level = request.approval_level.unwrap_or_else(|| {
            if scheduled_for.is_some() {
                "auto_read_only".to_string()
            } else {
                "confirm_before_execute".to_string()
            }
        });
        let approval_required = requires_approval(&approval_level);
        let status = if approval_required {
            "approval_needed"
        } else if scheduled_for.is_some() {
            "scheduled"
        } else {
            "queued"
        };
        let task = DispatchTask {
            id: Uuid::new_v4().to_string(),
            title: request.title,
            instruction: request.instruction,
            origin: request.origin,
            target: request.target,
            client_id: client.id.clone(),
            status: status.to_string(),
            approval_level,
            approval_required,
            scheduled_for,
            context: request.context,
            created_at: Utc::now(),
            approved_at: None,
            started_at: None,
            completed_at: None,
            completion_summary: None,
            evidence_refs: Vec::new(),
            failure_packet: None,
        };
        state.tasks.insert(0, task.clone());
        state.events.insert(
            0,
            event(
                &task.id,
                status,
                if approval_required {
                    "Task needs approval before dispatch"
                } else if task.scheduled_for.is_some() {
                    "Task scheduled for later dispatch"
                } else {
                    "Task queued for dispatch"
                },
                approval_required,
            ),
        );
        self.save_state(&state)?;
        Ok(task)
    }

    pub fn approve_task(&self, task_id: &str) -> Result<DispatchTask> {
        self.update_task(task_id, |task| {
            task.approved_at = Some(Utc::now());
            task.approval_required = false;
            if task.status == "approval_needed" {
                task.status = if task.scheduled_for.is_some() {
                    "scheduled".to_string()
                } else {
                    "queued".to_string()
                };
            }
            Ok(event(
                &task.id,
                "approved",
                "Task approved for dispatch",
                false,
            ))
        })
    }

    pub fn complete_task(&self, request: DispatchCompletionRequest) -> Result<DispatchTask> {
        if request.summary.trim().is_empty() {
            return Err(anyhow!("completion summary is required"));
        }
        if request.evidence_refs.is_empty() {
            return Err(anyhow!("completion evidence_refs are required"));
        }
        self.update_task(&request.task_id, |task| {
            task.status = "completed".to_string();
            task.completed_at = Some(Utc::now());
            task.completion_summary = Some(request.summary.clone());
            task.evidence_refs = request.evidence_refs.clone();
            Ok(event(
                &task.id,
                "completed",
                "Task completed with evidence",
                true,
            ))
        })
    }

    pub fn fail_task(&self, request: DispatchFailureRequest) -> Result<DispatchTask> {
        if request.error.trim().is_empty() {
            return Err(anyhow!("failure error is required"));
        }
        self.update_task(&request.task_id, |task| {
            task.status = "failed".to_string();
            task.completed_at = Some(Utc::now());
            task.failure_packet = Some(DispatchFailurePacket {
                error: request.error.clone(),
                retry_hint: request.retry_hint.clone(),
                log_refs: request.log_refs.clone(),
                created_at: Utc::now(),
            });
            Ok(event(
                &task.id,
                "failed",
                "Task failed; failure packet is available",
                true,
            ))
        })
    }

    pub fn status(&self, task_id: &str) -> Result<Option<DispatchMobileStatus>> {
        let state = self.state()?;
        let Some(task) = state.tasks.iter().find(|task| task.id == task_id) else {
            return Ok(None);
        };
        Ok(Some(status_from_task(task, &state.events)))
    }

    pub fn list_tasks(&self, limit: usize) -> Result<Vec<DispatchMobileStatus>> {
        let state = self.state()?;
        Ok(state
            .tasks
            .iter()
            .take(limit.max(1))
            .map(|task| status_from_task(task, &state.events))
            .collect())
    }

    pub fn watch_events(&self, task_id: Option<&str>, limit: usize) -> Result<Vec<DispatchEvent>> {
        Ok(self
            .state()?
            .events
            .into_iter()
            .filter(|event| task_id.map(|id| event.task_id == id).unwrap_or(true))
            .take(limit.max(1))
            .collect())
    }

    pub fn due_tasks(&self, as_of: &str) -> Result<Vec<DispatchTask>> {
        let as_of = DateTime::parse_from_rfc3339(as_of)?.with_timezone(&Utc);
        Ok(self
            .state()?
            .tasks
            .into_iter()
            .filter(|task| task.status == "scheduled")
            .filter(|task| {
                task.scheduled_for
                    .map(|time| time <= as_of)
                    .unwrap_or(false)
            })
            .collect())
    }

    fn update_task(
        &self,
        task_id: &str,
        update: impl FnOnce(&mut DispatchTask) -> Result<DispatchEvent>,
    ) -> Result<DispatchTask> {
        let mut state = self.state()?;
        let index = state
            .tasks
            .iter()
            .position(|task| task.id == task_id)
            .ok_or_else(|| anyhow!("unknown dispatch task {}", task_id))?;
        let event = update(&mut state.tasks[index])?;
        let task = state.tasks[index].clone();
        state.events.insert(0, event);
        self.save_state(&state)?;
        Ok(task)
    }
}

fn authenticate(state: &RemoteDispatchState, token: &str) -> Result<DispatchClient> {
    let hash = hash_token(token);
    state
        .clients
        .iter()
        .find(|client| client.token_hash == hash && client.revoked_at.is_none())
        .cloned()
        .ok_or_else(|| anyhow!("invalid dispatch token"))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn requires_approval(level: &str) -> bool {
    let level = level.to_ascii_lowercase();
    level.contains("confirm") || level.contains("approval")
}

fn parse_optional_time(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|parsed| parsed.with_timezone(&Utc))
                .with_context(|| format!("parse scheduled_for {}", value))
        })
        .transpose()
}

fn event(task_id: &str, kind: &str, message: &str, notify_user: bool) -> DispatchEvent {
    DispatchEvent {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        kind: kind.to_string(),
        message: message.to_string(),
        notify_user,
        created_at: Utc::now(),
    }
}

fn status_from_task(task: &DispatchTask, events: &[DispatchEvent]) -> DispatchMobileStatus {
    DispatchMobileStatus {
        task_id: task.id.clone(),
        title: task.title.clone(),
        status: task.status.clone(),
        approval_required: task.approval_required,
        approval_level: task.approval_level.clone(),
        scheduled_for: task.scheduled_for,
        evidence_refs: task.evidence_refs.clone(),
        failure_packet: task.failure_packet.clone(),
        last_event: events
            .iter()
            .find(|event| event.task_id == task.id)
            .map(|event| event.kind.clone()),
    }
}

fn require_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{} is required", label));
    }
    Ok(())
}
