//! IPC message types for the iAgent named-pipe protocol.
//!
//! All messages are serialized as compact JSON, one object per line (\n).
//! The pipe is full-duplex: the client writes requests, the server streams events.

use serde::{Deserialize, Serialize};

// ── Requests (client → server) ────────────────────────────────────────────────

/// A task the user wants the agent to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    /// Client-generated UUID v4 identifying this task.
    pub id: String,
    /// The user's natural-language prompt.
    pub prompt: String,
    /// Optional list of file paths to include as context.
    #[serde(default)]
    pub context_files: Vec<String>,
    /// Optional provider override (e.g. "openai", "openrouter"). Uses
    /// default_provider from settings if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Cancel a running task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub task_id: String,
}

/// Request the current system status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRequest {}

/// Top-level client message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Task(TaskRequest),
    Cancel(CancelRequest),
    Status(StatusRequest),
}

// ── Events (server → client) ──────────────────────────────────────────────────

/// The agent is thinking / planning (no output yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingEvent {
    pub task_id: String,
}

/// The agent produced a text chunk (streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEvent {
    pub task_id: String,
    /// Incremental text chunk. Clients should append to their display buffer.
    pub chunk: String,
}

/// The agent is invoking a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseEvent {
    pub task_id: String,
    pub tool_name: String,
    /// JSON-encoded tool input (opaque to the UI layer).
    pub input_json: String,
}

/// A tool call returned a result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultEvent {
    pub task_id: String,
    pub tool_name: String,
    /// Brief human-readable summary (not the full output).
    pub summary: String,
}

/// The task completed successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoneEvent {
    pub task_id: String,
    /// Total tokens used, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,
}

/// The task failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub task_id: String,
    pub message: String,
    /// Machine-readable error code for the UI to act on.
    pub code: ErrorCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ProviderError,
    AuthRequired,
    RateLimited,
    ContextTooLong,
    Cancelled,
    Internal,
}

/// Response to a StatusRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEvent {
    pub version: String,
    pub active_tasks: u32,
    pub default_provider: String,
    pub providers_available: Vec<String>,
}

/// Top-level server event envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Thinking(ThinkingEvent),
    Text(TextEvent),
    ToolUse(ToolUseEvent),
    ToolResult(ToolResultEvent),
    Done(DoneEvent),
    Error(ErrorEvent),
    Status(StatusEvent),
}
