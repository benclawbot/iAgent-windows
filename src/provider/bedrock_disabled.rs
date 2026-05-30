use super::{DEFAULT_CONTEXT_LIMIT, EventStream, ModelRoute, Provider};
use crate::message::{Message, ToolDefinition};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

const DEFAULT_MODEL: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";

pub const ENV_FILE: &str = "bedrock.env";
pub const API_KEY_ENV: &str = "AWS_BEARER_TOKEN_BEDROCK";
pub const REGION_ENV: &str = "IAGENT_BEDROCK_REGION";

pub struct BedrockProvider;

impl BedrockProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn has_credentials() -> bool {
        false
    }

    pub fn configured_bearer_token() -> Option<String> {
        None
    }

    pub fn is_bedrock_model_id(_model: &str) -> bool {
        false
    }

    fn disabled_error() -> anyhow::Error {
        anyhow::anyhow!(
            "AWS Bedrock provider is not compiled into this build. Rebuild with `--features bedrock` to enable it."
        )
    }
}

impl Default for BedrockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        Err(Self::disabled_error())
    }

    fn name(&self) -> &str {
        "bedrock"
    }

    fn model(&self) -> String {
        DEFAULT_MODEL.to_string()
    }

    fn set_model(&self, _model: &str) -> Result<()> {
        Err(Self::disabled_error())
    }

    fn available_models_for_switching(&self) -> Vec<String> {
        Vec::new()
    }

    fn model_routes(&self) -> Vec<ModelRoute> {
        Vec::new()
    }

    async fn prefetch_models(&self) -> Result<()> {
        Ok(())
    }

    fn context_window(&self) -> usize {
        DEFAULT_CONTEXT_LIMIT
    }

    fn uses_iagent_compaction(&self) -> bool {
        false
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self::new())
    }
}
