//! Skill script tool - executes scripts from skill directories
//!
//! When a tool is called with name "skillname_scriptname", this tool handles it
//! by reading the script from the skill's scripts/ directory and executing it.

use super::{Tool, ToolContext, ToolOutput};
use crate::skill::SkillRegistry;
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SkillScriptTool {
    registry: Arc<RwLock<SkillRegistry>>,
}

impl SkillScriptTool {
    pub fn new(registry: Arc<RwLock<SkillRegistry>>) -> Self {
        Self { registry }
    }

    /// Parse skill name and script name from a tool call name
    /// Returns None if the name doesn't match the pattern
    fn parse_tool_name(name: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = name.splitn(2, '_').collect();
        if parts.len() != 2 {
            return None;
        }
        Some((parts[0].to_string(), parts[1].to_string()))
    }
}

#[derive(Deserialize)]
struct ScriptInput {
    /// Skill name (required)
    skill: String,
    /// Script name (required)
    script: String,
    /// Arguments to pass to the script
    #[serde(default)]
    args: Option<String>,
}

#[async_trait]
impl Tool for SkillScriptTool {
    fn name(&self) -> &str {
        "skill_script"
    }

    fn description(&self) -> &str {
        "Execute a script from a skill's scripts directory"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intent": super::intent_schema_property(),
                "skill": {
                    "type": "string",
                    "description": "Skill name"
                },
                "script": {
                    "type": "string",
                    "description": "Script name to execute"
                },
                "args": {
                    "type": "string",
                    "description": "Arguments to pass to the script"
                }
            },
            "required": ["skill", "script"]
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let params: ScriptInput = serde_json::from_value(input)?;

        let registry = self.registry.read().await;
        let skill = registry
            .get(&params.skill)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", params.skill))?;

        let script_path = skill
            .scripts_dir()
            .ok_or_else(|| anyhow::anyhow!("No scripts directory for skill '{}'", params.skill))?
            .join(&params.script);

        if !script_path.exists() {
            return Err(anyhow::anyhow!(
                "Script '{}' not found in skill '{}' at {}",
                params.script,
                params.skill,
                script_path.display()
            ));
        }

        let script_content = std::fs::read_to_string(&script_path)?;

        let args = params.args.unwrap_or_default();
        let command = if args.is_empty() {
            script_content
        } else {
            format!("{} {}", script_content, args)
        };

        let working_dir = skill.scripts_dir().map(|p| p.display().to_string());
        super::bash::BashTool::new()
            .execute(
                json!({
                    "command": command,
                    "workingDir": working_dir
                }),
                ctx,
            )
            .await
    }
}
