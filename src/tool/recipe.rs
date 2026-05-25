use super::{Tool, ToolContext, ToolOutput};
use crate::recipe_catalog::{RecipeCatalog, RecipeDispatchRequest, RecipeInputValue, RecipeSearch};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct RecipeTool;

impl RecipeTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct RecipeInput {
    action: String,
    #[serde(default)]
    recipe_id: Option<String>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    inputs: Vec<RecipeInputValue>,
}

#[async_trait]
impl Tool for RecipeTool {
    fn name(&self) -> &str {
        "recipe"
    }

    fn description(&self) -> &str {
        "Search hotkey-ready workflow recipes and create typed dispatch plans for common iAgent workflows without executing them."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": super::intent_schema_property(),
                "action": {
                    "type": "string",
                    "enum": ["list", "search", "get", "plan"],
                    "description": "Recipe catalog action."
                },
                "recipe_id": {
                    "type": "string",
                    "description": "Recipe identifier for get/plan."
                },
                "query": {
                    "type": "string",
                    "description": "Search query."
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tags all returned recipes must include."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of recipes to return."
                },
                "inputs": {
                    "type": "array",
                    "description": "Typed recipe input values for plan.",
                    "items": {
                        "type": "object",
                        "required": ["name", "value"],
                        "properties": {
                            "name": {"type": "string"},
                            "value": {"type": "string"}
                        }
                    }
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: RecipeInput = serde_json::from_value(input)?;
        let catalog = RecipeCatalog::built_in();

        match input.action.as_str() {
            "list" => Ok(
                ToolOutput::new(serde_json::to_string_pretty(catalog.all())?)
                    .with_title(format!("{} recipes", catalog.all().len())),
            ),
            "search" => {
                let results = catalog.search(RecipeSearch {
                    query: input.query,
                    tags: input.tags,
                    limit: input.limit.unwrap_or(10),
                });
                Ok(ToolOutput::new(serde_json::to_string_pretty(&results)?)
                    .with_title(format!("{} recipe matches", results.len())))
            }
            "get" => {
                let recipe_id = input
                    .recipe_id
                    .ok_or_else(|| anyhow!("recipe_id is required"))?;
                let recipe = catalog
                    .get(&recipe_id)
                    .ok_or_else(|| anyhow!("unknown recipe {}", recipe_id))?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(recipe)?)
                    .with_title(recipe.title.clone()))
            }
            "plan" => {
                let recipe_id = input
                    .recipe_id
                    .ok_or_else(|| anyhow!("recipe_id is required"))?;
                let plan = catalog.dispatch_plan(RecipeDispatchRequest {
                    recipe_id,
                    inputs: input.inputs,
                })?;
                Ok(ToolOutput::new(serde_json::to_string_pretty(&plan)?)
                    .with_title(format!("Recipe plan: {}", plan.title))
                    .with_metadata(json!({ "recipe_plan": plan })))
            }
            other => Err(anyhow!("unsupported recipe action '{}'", other)),
        }
    }
}
