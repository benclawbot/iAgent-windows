//! Goal judge routing layer.
//!
//! Routes tool execution requests based on goal context before hitting
//! the actual tool execution in `Registry::execute()`.

use crate::config::config;
use crate::goal;
use crate::provider::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Routing decision from goal judge - controls whether a tool proceeds,
/// redirects to an alternative, or is blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum RoutingDecision {
    /// Tool execution should proceed normally.
    Proceed,
    /// Redirect to an alternative tool/workflow with a reason.
    Redirect { alternative: String, reason: String },
    /// Block the tool execution with a reason.
    Block { reason: String },
}

impl RoutingDecision {
    /// Returns true if this decision allows proceeding.
    pub fn is_proceed(&self) -> bool {
        matches!(self, RoutingDecision::Proceed)
    }

    /// Returns the reason if blocked.
    pub fn block_reason(&self) -> Option<&str> {
        match self {
            RoutingDecision::Block { reason } => Some(reason),
            _ => None,
        }
    }

    /// Returns (alternative, reason) if redirect.
    pub fn redirect_info(&self) -> Option<(&str, &str)> {
        match self {
            RoutingDecision::Redirect {
                alternative,
                reason,
            } => Some((alternative, reason)),
            _ => None,
        }
    }
}

/// GoalJudge configuration for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GoalJudgeConfig {
    /// Enable goal-based tool routing (default: false).
    pub enabled: bool,
    /// Optional model override for goal routing decisions.
    pub model: Option<String>,
}

impl Default for GoalJudgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: None,
        }
    }
}

/// GoalJudge routes tool execution based on active goal context.
#[derive(Debug, Clone)]
pub struct GoalJudge {
    config: GoalJudgeConfig,
}

impl GoalJudge {
    /// Create a new GoalJudge from config.
    pub fn new() -> Self {
        Self {
            config: config().goal_judge.clone(),
        }
    }

    /// Check if goal_judge routing is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Make a routing decision by calling the model.
    /// Returns Proceed if no goal context or routing is disabled.
    pub async fn make_routing_decision(
        &self,
        provider: Arc<dyn Provider>,
        tool_name: &str,
        input: &Value,
        session_id: &str,
        working_dir: Option<&std::path::Path>,
    ) -> Result<RoutingDecision> {
        if !self.is_enabled() {
            return Ok(RoutingDecision::Proceed);
        }

        // Load the attached goal for this session to get context
        let active_goal = match goal::load_attached_goal(session_id, working_dir)? {
            Some(g) => g,
            None => return Ok(RoutingDecision::Proceed),
        };

        // Build the routing prompt
        let input_preview = if input.is_object() {
            serde_json::to_string(input).unwrap_or_else(|_| "<invalid JSON>".to_string())
        } else {
            format!("{}", input)
        };

        let prompt = format!(
            r#"You are a goal-aware routing judge. Analyze the following tool call in context of the active goal.

ACTIVE GOAL:
- ID: {}
- Title: {}
- Status: {}

TOOL CALL:
- Tool: {}
- Input: {}

Evaluate if this tool call should:
1. PROCEED - aligns with the goal, proceed normally
2. REDIRECT - partial alignment but an alternative approach would be better
3. BLOCK - conflicts with or undermines the goal

Respond with JSON in this exact format:
{{"decision": "proceed"}} - if should proceed
{{"decision": "redirect", "alternative": "<tool or approach>", "reason": "<why redirect>"}} - if should redirect
{{"decision": "block", "reason": "<why blocked>"}} - if should be blocked

Only output the JSON, no additional text."#,
            active_goal.id,
            active_goal.title,
            active_goal.status.as_str(),
            tool_name,
            input_preview
        );

        let response = provider.complete_simple(&prompt, "").await?;
        let response = response.trim();

        // Parse the model's JSON response
        let parsed: serde_json::Value = serde_json::from_str(response)
            .unwrap_or_else(|_| serde_json::json!({"decision": "proceed"}));

        let decision = parsed
            .get("decision")
            .and_then(|v| v.as_str())
            .unwrap_or("proceed");

        match decision {
            "redirect" => {
                let alternative = parsed
                    .get("alternative")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let reason = parsed
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("redirected by goal judge")
                    .to_string();
                Ok(RoutingDecision::Redirect {
                    alternative,
                    reason,
                })
            }
            "block" => {
                let reason = parsed
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("blocked by goal judge")
                    .to_string();
                Ok(RoutingDecision::Block { reason })
            }
            _ => Ok(RoutingDecision::Proceed),
        }
    }

    /// Determine if a tool should be routed based on goal context.
    /// Returns Ok(None) if routing should proceed normally,
    /// Ok(Some(routed_output)) if the tool was handled by goal routing,
    /// or Err if there's an error in the routing decision.
    pub fn route_tool(
        &self,
        tool_name: &str,
        _input: &Value,
        session_id: &str,
        working_dir: Option<&std::path::Path>,
    ) -> Result<Option<goal::Goal>> {
        if !self.is_enabled() {
            return Ok(None);
        }

        // Only route "goal" tool calls - check if there's an active goal
        if tool_name != "goal" {
            return Ok(None);
        }

        // Load the attached goal for this session to get context
        let active_goal = match goal::load_attached_goal(session_id, working_dir)? {
            Some(g) => g,
            None => return Ok(None),
        };

        // For goal tool calls, return the active goal for context-aware handling
        // The actual goal tool will use this context to validate/route the request
        Ok(Some(active_goal))
    }

    /// Validate a tool execution request against goal context.
    /// Returns Ok(()) if allowed, Err(reason) if blocked.
    pub fn validate_tool_for_goal(
        &self,
        tool_name: &str,
        _input: &Value,
        session_id: &str,
        working_dir: Option<&std::path::Path>,
    ) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        // Get active goal for this session
        let Some(active_goal) = goal::load_attached_goal(session_id, working_dir)? else {
            return Ok(()); // No active goal, allow tool execution
        };

        // If there's an active goal, log the tool execution context
        log_info!((
            "[goal_judge] routing tool '{}' with active goal id={} title=\"{}\"",
            tool_name,
            active_goal.id,
            active_goal.title
        ));

        Ok(())
    }
}

impl Default for GoalJudge {
    fn default() -> Self {
        Self::new()
    }
}
