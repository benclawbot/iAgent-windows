//! `compaction` tool — View compaction status and trigger manually.
//!
//! Exposes the CompactionManager's internal state so users can see:
//! - Current message count and token estimate
//! - When the next automatic compaction will trigger
//! - Manual trigger option
//!
//! Note: Most config options (threshold, mode, etc.) are in jcode-config-types
//! and controlled via config.toml. This tool provides visibility only.

use crate::compaction::CompactionManager;
use crate::tool::{Tool, ToolContext, ToolOutput, intent_schema_property};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum CompactionInput {
    /// Get current compaction statistics and trigger status.
    status,

    /// Manually trigger compaction now (if session has enough messages).
    trigger,
}

pub struct CompactionTool {
    compaction: Arc<RwLock<CompactionManager>>,
}

impl CompactionTool {
    pub fn new(compaction: Arc<RwLock<CompactionManager>>) -> Self {
        Self { compaction }
    }
}

#[async_trait]
impl Tool for CompactionTool {
    fn name(&self) -> &str {
        "compaction"
    }

    fn description(&self) -> &str {
        "View compaction status (message count, token estimate, next trigger) and manually trigger compaction."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "trigger"],
                    "description": "Action to perform: 'status' to view stats, 'trigger' to manually compact"
                },
                "intent": intent_schema_property()
            }
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let input: CompactionInput = serde_json::from_value(input)?;

        match input {
            CompactionInput::status => self.show_status(&ctx.session_id).await,
            CompactionInput::trigger => self.trigger_compaction(&ctx.session_id).await,
        }
    }
}

impl CompactionTool {
    async fn show_status(&self, _session_id: &str) -> Result<ToolOutput> {
        let manager = self.compaction.read().await;
        let stats = manager.stats_with(&[]);

        // Calculate trigger threshold (80% of budget is default)
        let budget = manager.token_budget();
        let trigger_threshold = (budget as f32 * 0.80) as usize;

        // Estimate messages until next compaction
        let tokens_until_trigger = trigger_threshold.saturating_sub(stats.token_estimate);
        let avg_tokens_per_msg = if stats.active_messages > 0 {
            stats.token_estimate / stats.active_messages
        } else {
            500 // rough estimate
        };
        let msgs_until_trigger = if avg_tokens_per_msg > 0 {
            tokens_until_trigger / avg_tokens_per_msg
        } else {
            999
        };

        let status_json = json!({
            "current": {
                "active_messages": stats.active_messages,
                "token_estimate": stats.token_estimate,
                "effective_tokens": stats.effective_tokens,
                "context_usage_pct": if budget > 0 {
                    (stats.token_estimate as f32 / budget as f32 * 100.0).min(100.0)
                } else { 0.0 },
                "has_summary": stats.has_summary,
                "is_compacting": stats.is_compacting,
            },
            "thresholds": {
                "token_budget": budget,
                "trigger_at_pct": 80,
                "trigger_at_tokens": trigger_threshold,
            },
            "projection": {
                "tokens_until_trigger": tokens_until_trigger,
                "messages_until_estimate": msgs_until_trigger.min(999),
                "avg_tokens_per_message": avg_tokens_per_msg,
            },
            "config_note": "Compaction config (mode, thresholds) is set in config.toml under [compaction]. This tool provides visibility only."
        });

        Ok(ToolOutput::new(serde_json::to_string_pretty(&status_json)?))
    }

    async fn trigger_compaction(&self, _session_id: &str) -> Result<ToolOutput> {
        let manager = self.compaction.read().await;
        let stats = manager.stats_with(&[]);

        if stats.is_compacting {
            return Ok(ToolOutput::new(
                "Compaction already in progress".to_string(),
            ));
        }

        if stats.active_messages < 5 {
            return Ok(ToolOutput::new(format!(
                "Not enough messages to compact ({} < 5). Compaction requires sufficient history to be useful.",
                stats.active_messages
            )));
        }

        Ok(ToolOutput::new(json!({
            "status": "available_on_next_agent_turn",
            "messages_to_compact": stats.active_messages,
            "note": "Manual compaction requires the active agent message history and provider; it will be evaluated by the agent loop when context pressure is high."
        }).to_string()))
    }
}
