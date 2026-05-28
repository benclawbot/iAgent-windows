//! Types for IPC between iAgent Windows shell and backend process.
//!
//! All types are serde-serializable with no async, no tokio, and no heavy deps.
//! Used for communication over Windows named pipes.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Session management
// =============================================================================

/// Unique identifier for an iAgent session.
pub type SessionId = Uuid;

// =============================================================================
// Shell → Backend (requests)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Request {
    #[serde(rename = "SuggestionRequest")]
    SuggestionRequest(SuggestionRequestPayload),

    #[serde(rename = "GetActiveWindow")]
    GetActiveWindow,

    #[serde(rename = "ConfigGet")]
    ConfigGet(ConfigGetPayload),

    #[serde(rename = "ConfigSet")]
    ConfigSet(ConfigSetPayload),

    #[serde(rename = "HealthCheck")]
    HealthCheck,

    #[serde(rename = "Shutdown")]
    Shutdown,

    #[serde(rename = "ReloadProvider")]
    ReloadProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionRequestPayload {
    pub session_id: SessionId,
    pub text: String,
    pub window_context: WindowContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowContext {
    pub app_name: String,
    pub window_title: String,
    pub context_type: ContextType,
    pub text_content: Option<String>,
    pub cursor_position: (i32, i32),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextType {
    Email,
    Document,
    Presentation,
    Code,
    Chat,
    Browser,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigGetPayload {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSetPayload {
    pub key: String,
    pub value: serde_json::Value,
}

// =============================================================================
// Backend → Shell (responses)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Response {
    #[serde(rename = "SuggestionResponse")]
    SuggestionResponse(SuggestionResponsePayload),

    #[serde(rename = "ActiveWindow")]
    ActiveWindow(WindowContext),

    #[serde(rename = "ConfigValue")]
    ConfigValue(ConfigValuePayload),

    #[serde(rename = "HealthCheck")]
    HealthCheck(HealthCheckPayload),

    #[serde(rename = "ShutdownAck")]
    ShutdownAck,

    #[serde(rename = "ReloadProviderAck")]
    ReloadProviderAck,

    #[serde(rename = "Error")]
    Error(ErrorPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResponsePayload {
    pub session_id: SessionId,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub variant_label: String,
    pub text: String,
    pub reasoning: Option<String>,
    pub intent: Option<SuggestionIntent>,
    pub confidence: Option<f32>,
    pub action: Option<ActionCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionIntent {
    SummarizeEmail,
    DraftReply,
    ExtractTasks,
    PrepareJiraTicket,
    BuildSlideOutline,
    FillForm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCard {
    pub intent: SuggestionIntent,
    pub confidence: f32,
    pub risk_level: String,
    pub approval_required: bool,
    pub required_inputs: Vec<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValuePayload {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckPayload {
    pub backend_version: String,
    pub providers_ready: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}
