use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConnectorPackStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ConnectorPackState {
    #[serde(default)]
    pub grants: Vec<ConnectorGrant>,
    #[serde(default)]
    pub write_evidence: Vec<ConnectorWriteEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorDefinition {
    pub id: String,
    pub pack_id: String,
    pub pack_name: String,
    pub display_name: String,
    pub description: String,
    pub auth_kind: String,
    #[serde(default)]
    pub scopes: Vec<ConnectorScopeDefinition>,
    #[serde(default)]
    pub write_operations: Vec<ConnectorWriteOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorScopeDefinition {
    pub id: String,
    pub access: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorWriteOperation {
    pub id: String,
    pub label: String,
    pub required_scopes: Vec<String>,
    pub approval_level: String,
    pub evidence_required: bool,
}

#[derive(Debug, Clone)]
pub struct ConnectorGrantRequest {
    pub connector_id: String,
    pub scopes: Vec<String>,
    pub actor: String,
    pub reason: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorGrant {
    pub id: String,
    pub connector_id: String,
    pub scopes: Vec<String>,
    pub actor: String,
    pub reason: String,
    pub granted_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ConnectorWritePreflight {
    pub connector_id: String,
    pub operation: String,
    pub target: String,
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorWriteDecision {
    pub id: String,
    pub connector_id: String,
    pub operation: String,
    pub target: String,
    pub allowed: bool,
    pub required_scopes: Vec<String>,
    pub missing_scopes: Vec<String>,
    pub grant_ids: Vec<String>,
    pub approval_level: String,
    pub evidence_required: bool,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ConnectorEvidenceInput {
    pub connector_id: String,
    pub operation: String,
    pub target: String,
    pub run_id: String,
    pub tool_call_id: Option<String>,
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorWriteEvidence {
    pub id: String,
    pub connector_id: String,
    pub operation: String,
    pub target: String,
    pub required_scopes: Vec<String>,
    pub grant_ids: Vec<String>,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub recorded_at: DateTime<Utc>,
}

impl ConnectorPackStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::iagent_dir()?.join("connectors");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("connector_packs.json"),
        })
    }

    pub fn built_in_catalog() -> Vec<ConnectorDefinition> {
        vec![
            connector(
                "outlook_mail",
                "microsoft_365",
                "Microsoft 365",
                "Outlook Mail",
                "Read, draft, and send Outlook mail through scoped Microsoft Graph permissions.",
                "oauth",
                vec![
                    scope(
                        "mail.read",
                        "read",
                        "Read mailbox metadata and message bodies.",
                    ),
                    scope(
                        "mail.write",
                        "write",
                        "Create drafts, update messages, and send mail.",
                    ),
                ],
                vec![
                    op("create_draft", "Create mail draft", vec!["mail.write"]),
                    op("send_email", "Send email", vec!["mail.write"]),
                ],
            ),
            connector(
                "outlook_calendar",
                "microsoft_365",
                "Microsoft 365",
                "Outlook Calendar",
                "Read, create, and update Outlook calendar events with explicit calendar scopes.",
                "oauth",
                vec![
                    scope(
                        "calendar.read",
                        "read",
                        "Read calendar events and availability.",
                    ),
                    scope(
                        "calendar.write",
                        "write",
                        "Create, update, or delete calendar events.",
                    ),
                ],
                vec![op(
                    "create_event",
                    "Create calendar event",
                    vec!["calendar.write"],
                )],
            ),
            connector(
                "gmail",
                "google_workspace",
                "Google Workspace",
                "Gmail",
                "Read, draft, label, and send Gmail messages through scoped Google OAuth grants.",
                "oauth",
                vec![
                    scope(
                        "mail.read",
                        "read",
                        "Read Gmail metadata and message bodies.",
                    ),
                    scope(
                        "mail.write",
                        "write",
                        "Create drafts, labels, and send mail.",
                    ),
                ],
                vec![
                    op("create_draft", "Create mail draft", vec!["mail.write"]),
                    op("send_email", "Send email", vec!["mail.write"]),
                ],
            ),
            connector(
                "google_calendar",
                "google_workspace",
                "Google Workspace",
                "Google Calendar",
                "Read, create, and update Google Calendar events with explicit calendar scopes.",
                "oauth",
                vec![
                    scope(
                        "calendar.read",
                        "read",
                        "Read calendars, events, and availability.",
                    ),
                    scope(
                        "calendar.write",
                        "write",
                        "Create, update, or delete events.",
                    ),
                ],
                vec![op(
                    "create_event",
                    "Create calendar event",
                    vec!["calendar.write"],
                )],
            ),
            connector(
                "slack",
                "team_chat",
                "Team Chat",
                "Slack",
                "Read channels and post approved messages with scoped workspace permissions.",
                "oauth",
                vec![
                    scope(
                        "channels.read",
                        "read",
                        "Read channel names, membership, and history.",
                    ),
                    scope("messages.write", "write", "Post or update Slack messages."),
                ],
                vec![op(
                    "post_message",
                    "Post channel message",
                    vec!["messages.write"],
                )],
            ),
            connector(
                "teams",
                "team_chat",
                "Team Chat",
                "Microsoft Teams",
                "Read chats and post approved Teams messages with scoped Microsoft Graph permissions.",
                "oauth",
                vec![
                    scope("chats.read", "read", "Read chat and channel context."),
                    scope("messages.write", "write", "Post or update Teams messages."),
                ],
                vec![op(
                    "post_message",
                    "Post chat or channel message",
                    vec!["messages.write"],
                )],
            ),
            connector(
                "github",
                "developer_work",
                "Developer Work",
                "GitHub",
                "Read repositories and create or update issues and pull requests with scoped grants.",
                "oauth_or_pat",
                vec![
                    scope(
                        "repos.read",
                        "read",
                        "Read repositories, files, issues, and pull requests.",
                    ),
                    scope("issues.write", "write", "Create or update issues."),
                    scope(
                        "pull_requests.write",
                        "write",
                        "Create or update pull requests.",
                    ),
                ],
                vec![
                    op("create_issue", "Create issue", vec!["issues.write"]),
                    op(
                        "update_pull_request",
                        "Update pull request",
                        vec!["pull_requests.write"],
                    ),
                ],
            ),
            connector(
                "linear",
                "developer_work",
                "Developer Work",
                "Linear",
                "Read and update Linear issues with explicit workspace scopes.",
                "oauth",
                vec![
                    scope("issues.read", "read", "Read teams, projects, and issues."),
                    scope("issues.write", "write", "Create or update Linear issues."),
                ],
                vec![op("create_issue", "Create issue", vec!["issues.write"])],
            ),
            connector(
                "jira",
                "developer_work",
                "Developer Work",
                "Jira",
                "Read and update Jira projects, issues, and comments with scoped Atlassian grants.",
                "oauth",
                vec![
                    scope(
                        "issues.read",
                        "read",
                        "Read projects, issues, and comments.",
                    ),
                    scope("issues.write", "write", "Create or update Jira issues."),
                ],
                vec![op("create_issue", "Create issue", vec!["issues.write"])],
            ),
            connector(
                "notion",
                "knowledge",
                "Knowledge",
                "Notion",
                "Read and update pages or databases with scoped workspace permissions.",
                "oauth",
                vec![
                    scope("pages.read", "read", "Read shared pages and blocks."),
                    scope("pages.write", "write", "Create or update pages and blocks."),
                    scope("databases.read", "read", "Read shared databases."),
                    scope(
                        "databases.write",
                        "write",
                        "Create or update database rows.",
                    ),
                ],
                vec![
                    op("update_page", "Update page", vec!["pages.write"]),
                    op(
                        "create_database_item",
                        "Create database item",
                        vec!["databases.write"],
                    ),
                ],
            ),
            connector(
                "obsidian",
                "knowledge",
                "Knowledge",
                "Obsidian",
                "Read and write local vault notes through explicit vault scopes.",
                "local",
                vec![
                    scope(
                        "vault.read",
                        "read",
                        "Read notes from approved vault paths.",
                    ),
                    scope(
                        "vault.write",
                        "write",
                        "Create or update notes in approved vault paths.",
                    ),
                ],
                vec![op(
                    "write_note",
                    "Create or update note",
                    vec!["vault.write"],
                )],
            ),
            connector(
                "file_share",
                "files",
                "File Shares",
                "File Share",
                "Read and write approved local, SMB, OneDrive, or SharePoint paths with file scopes.",
                "local_or_oauth",
                vec![
                    scope(
                        "files.read",
                        "read",
                        "Read approved folders and file metadata.",
                    ),
                    scope(
                        "files.write",
                        "write",
                        "Create, update, move, or delete approved files.",
                    ),
                ],
                vec![op(
                    "write_file",
                    "Create or update shared file",
                    vec!["files.write"],
                )],
            ),
        ]
    }

    pub fn state(&self) -> Result<ConnectorPackState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read connector pack state at {}", self.path.display()))
        } else {
            Ok(ConnectorPackState::default())
        }
    }

    fn save_state(&self, state: &ConnectorPackState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write connector pack state at {}", self.path.display()))
    }

    pub fn get_connector(connector_id: &str) -> Result<ConnectorDefinition> {
        Self::built_in_catalog()
            .into_iter()
            .find(|connector| connector.id == connector_id)
            .ok_or_else(|| anyhow!("unknown connector {}", connector_id))
    }

    pub fn grant_scopes(&self, request: ConnectorGrantRequest) -> Result<ConnectorGrant> {
        let connector = Self::get_connector(&request.connector_id)?;
        let scopes = normalize_scopes(request.scopes)?;
        validate_scopes(&connector, &scopes)?;
        if request.actor.trim().is_empty() {
            return Err(anyhow!("actor is required"));
        }
        if request.reason.trim().is_empty() {
            return Err(anyhow!("reason is required"));
        }
        let expires_at = parse_optional_time(request.expires_at)?;
        let mut state = self.state()?;
        let grant = ConnectorGrant {
            id: Uuid::new_v4().to_string(),
            connector_id: connector.id,
            scopes,
            actor: request.actor,
            reason: request.reason,
            granted_at: Utc::now(),
            expires_at,
            revoked_at: None,
        };
        state.grants.insert(0, grant.clone());
        self.save_state(&state)?;
        Ok(grant)
    }

    pub fn revoke_grant(&self, grant_id: &str) -> Result<bool> {
        let mut state = self.state()?;
        let mut found = false;
        for grant in &mut state.grants {
            if grant.id == grant_id && grant.revoked_at.is_none() {
                grant.revoked_at = Some(Utc::now());
                found = true;
            }
        }
        if found {
            self.save_state(&state)?;
        }
        Ok(found)
    }

    pub fn list_grants(
        &self,
        connector_id: Option<&str>,
        active_only: bool,
    ) -> Result<Vec<ConnectorGrant>> {
        let now = Utc::now();
        Ok(self
            .state()?
            .grants
            .into_iter()
            .filter(|grant| {
                connector_id
                    .map(|connector_id| grant.connector_id == connector_id)
                    .unwrap_or(true)
            })
            .filter(|grant| !active_only || grant_is_active(grant, now))
            .collect())
    }

    pub fn preflight_write(
        &self,
        request: ConnectorWritePreflight,
    ) -> Result<ConnectorWriteDecision> {
        let connector = Self::get_connector(&request.connector_id)?;
        let operation = connector
            .write_operations
            .iter()
            .find(|operation| operation.id == request.operation)
            .ok_or_else(|| {
                anyhow!(
                    "connector {} does not expose write operation {}",
                    request.connector_id,
                    request.operation
                )
            })?;
        if request.target.trim().is_empty() {
            return Err(anyhow!("target is required"));
        }

        let now = Utc::now();
        let active_grants = self.list_grants(Some(&connector.id), true)?;
        let granted_scopes: std::collections::HashSet<String> = active_grants
            .iter()
            .flat_map(|grant| grant.scopes.iter().cloned())
            .collect();
        let missing_scopes: Vec<String> = operation
            .required_scopes
            .iter()
            .filter(|scope| !granted_scopes.contains(*scope))
            .cloned()
            .collect();
        let grant_ids: Vec<String> = active_grants
            .into_iter()
            .filter(|grant| {
                operation
                    .required_scopes
                    .iter()
                    .all(|scope| grant.scopes.contains(scope))
            })
            .map(|grant| grant.id)
            .collect();
        let allowed = missing_scopes.is_empty();
        Ok(ConnectorWriteDecision {
            id: Uuid::new_v4().to_string(),
            connector_id: connector.id,
            operation: operation.id.clone(),
            target: request.target,
            allowed,
            required_scopes: operation.required_scopes.clone(),
            missing_scopes,
            grant_ids,
            approval_level: operation.approval_level.clone(),
            evidence_required: operation.evidence_required,
            reason: if allowed {
                "required write scopes are active".to_string()
            } else {
                "missing active write scopes".to_string()
            },
            run_id: request.run_id,
            checked_at: now,
        })
    }

    pub fn record_write_evidence(
        &self,
        input: ConnectorEvidenceInput,
    ) -> Result<ConnectorWriteEvidence> {
        if input.run_id.trim().is_empty() {
            return Err(anyhow!("run_id is required"));
        }
        if input.summary.trim().is_empty() {
            return Err(anyhow!("summary is required"));
        }
        if input.evidence_refs.is_empty() {
            return Err(anyhow!("at least one evidence_ref is required"));
        }
        let decision = self.preflight_write(ConnectorWritePreflight {
            connector_id: input.connector_id,
            operation: input.operation,
            target: input.target,
            run_id: Some(input.run_id.clone()),
        })?;
        if !decision.allowed {
            return Err(anyhow!(
                "write is not allowed; missing scopes: {}",
                decision.missing_scopes.join(", ")
            ));
        }

        let mut state = self.state()?;
        let evidence = ConnectorWriteEvidence {
            id: Uuid::new_v4().to_string(),
            connector_id: decision.connector_id,
            operation: decision.operation,
            target: decision.target,
            required_scopes: decision.required_scopes,
            grant_ids: decision.grant_ids,
            run_id: input.run_id,
            tool_call_id: input.tool_call_id,
            summary: input.summary,
            evidence_refs: input.evidence_refs,
            recorded_at: Utc::now(),
        };
        state.write_evidence.insert(0, evidence.clone());
        self.save_state(&state)?;
        Ok(evidence)
    }

    pub fn audit_writes(
        &self,
        connector_id: Option<&str>,
        run_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ConnectorWriteEvidence>> {
        Ok(self
            .state()?
            .write_evidence
            .into_iter()
            .filter(|entry| {
                connector_id
                    .map(|connector_id| entry.connector_id == connector_id)
                    .unwrap_or(true)
            })
            .filter(|entry| run_id.map(|run_id| entry.run_id == run_id).unwrap_or(true))
            .take(limit.max(1))
            .collect())
    }
}

#[allow(clippy::too_many_arguments)]
fn connector(
    id: &str,
    pack_id: &str,
    pack_name: &str,
    display_name: &str,
    description: &str,
    auth_kind: &str,
    scopes: Vec<ConnectorScopeDefinition>,
    write_operations: Vec<ConnectorWriteOperation>,
) -> ConnectorDefinition {
    ConnectorDefinition {
        id: id.to_string(),
        pack_id: pack_id.to_string(),
        pack_name: pack_name.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        auth_kind: auth_kind.to_string(),
        scopes,
        write_operations,
    }
}

fn scope(id: &str, access: &str, description: &str) -> ConnectorScopeDefinition {
    ConnectorScopeDefinition {
        id: id.to_string(),
        access: access.to_string(),
        description: description.to_string(),
    }
}

fn op(id: &str, label: &str, required_scopes: Vec<&str>) -> ConnectorWriteOperation {
    ConnectorWriteOperation {
        id: id.to_string(),
        label: label.to_string(),
        required_scopes: required_scopes
            .into_iter()
            .map(|scope| scope.to_string())
            .collect(),
        approval_level: "confirm_before_write".to_string(),
        evidence_required: true,
    }
}

fn normalize_scopes(scopes: Vec<String>) -> Result<Vec<String>> {
    let mut scopes: Vec<String> = scopes
        .into_iter()
        .map(|scope| scope.trim().to_string())
        .filter(|scope| !scope.is_empty())
        .collect();
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty() {
        return Err(anyhow!("at least one scope is required"));
    }
    Ok(scopes)
}

fn validate_scopes(connector: &ConnectorDefinition, requested: &[String]) -> Result<()> {
    let available: std::collections::HashSet<&str> = connector
        .scopes
        .iter()
        .map(|scope| scope.id.as_str())
        .collect();
    for scope in requested {
        if !available.contains(scope.as_str()) {
            return Err(anyhow!(
                "connector {} does not expose scope {}",
                connector.id,
                scope
            ));
        }
    }
    Ok(())
}

fn parse_optional_time(value: Option<String>) -> Result<Option<DateTime<Utc>>> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|parsed| parsed.with_timezone(&Utc))
                .with_context(|| format!("parse expires_at {}", value))
        })
        .transpose()
}

fn grant_is_active(grant: &ConnectorGrant, now: DateTime<Utc>) -> bool {
    grant.revoked_at.is_none()
        && grant
            .expires_at
            .map(|expires| expires > now)
            .unwrap_or(true)
}
