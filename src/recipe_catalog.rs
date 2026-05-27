use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeCatalog {
    recipes: Vec<Recipe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recipe {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub hotkey: Option<String>,
    pub approval_policy: String,
    pub required_tools: Vec<String>,
    pub inputs: Vec<RecipeInputSpec>,
    pub steps: Vec<RecipeStepTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeInputSpec {
    pub name: String,
    pub label: String,
    pub kind: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeStepTemplate {
    pub tool: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeSearch {
    pub query: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeInputValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeDispatchRequest {
    pub recipe_id: String,
    #[serde(default)]
    pub inputs: Vec<RecipeInputValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeDispatchPlan {
    pub recipe_id: String,
    pub title: String,
    pub summary: String,
    pub approval_policy: String,
    pub required_tools: Vec<String>,
    pub steps: Vec<RecipeDispatchStep>,
    pub hotkey: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeDispatchStep {
    pub tool: String,
    pub action: String,
    pub description: String,
    pub input_summary: Option<String>,
}

impl RecipeCatalog {
    pub fn built_in() -> Self {
        Self {
            recipes: vec![
                recipe(
                    "folder_summary",
                    "Summarize folder",
                    "Summarize a local folder, identify important files, and prepare follow-up actions.",
                    &["files", "summary", "research"],
                    Some("Ctrl+Alt+F"),
                    "confirm_before_read",
                    &["personal", "read", "grep", "todo"],
                    vec![input(
                        "folder",
                        "Folder",
                        "path",
                        true,
                        "Folder path to summarize",
                    )],
                    vec![
                        step(
                            "personal",
                            "preview_redaction",
                            "Check folder goal text for sensitive context",
                        ),
                        step("grep", "search", "Find relevant documents and notes"),
                        step("read", "read", "Read selected files"),
                        step("todo", "write", "Create follow-up tasks from the summary"),
                    ],
                ),
                recipe(
                    "office_document",
                    "Create Office document",
                    "Create a Word, Excel, or PowerPoint artifact from a natural-language goal, then open the saved file.",
                    &["office", "document", "deliverable"],
                    Some("Ctrl+Alt+O"),
                    "confirm_before_write",
                    &["personal", "word", "open"],
                    vec![
                        input(
                            "goal",
                            "Goal",
                            "text",
                            true,
                            "Document goal or source notes",
                        ),
                        input(
                            "document_type",
                            "Document type",
                            "choice",
                            true,
                            "word, excel, or powerpoint",
                        ),
                    ],
                    vec![
                        step(
                            "personal",
                            "preview_redaction",
                            "Preview sensitive context before creating the artifact",
                        ),
                        step("word", "create", "Create the requested Office artifact"),
                        step("open", "open", "Open the saved artifact for inspection"),
                        step("flight_recorder", "view", "Capture the run evidence packet"),
                    ],
                ),
                recipe(
                    "web_form_fill",
                    "Fill web form",
                    "Open a web form, inspect its fields, fill from structured data, and stop for approval before submit.",
                    &["browser", "forms", "automation"],
                    Some("Ctrl+Alt+B"),
                    "confirm_before_external_submit",
                    &["personal", "browser", "flight_recorder"],
                    vec![
                        input("url", "URL", "url", true, "Form URL"),
                        input(
                            "data_source",
                            "Data source",
                            "text",
                            true,
                            "Structured data to fill",
                        ),
                    ],
                    vec![
                        step(
                            "personal",
                            "preview_redaction",
                            "Preview supplied form data for sensitive fields",
                        ),
                        step("browser", "navigate", "Open the form"),
                        step("browser", "inspect", "Extract fields and labels"),
                        step("browser", "fill", "Fill fields without submitting"),
                        step(
                            "flight_recorder",
                            "view",
                            "Summarize pending approval and evidence",
                        ),
                    ],
                ),
                recipe(
                    "meeting_prep",
                    "Prepare meeting brief",
                    "Collect relevant project context and create a concise meeting brief with questions and decisions needed.",
                    &["meeting", "brief", "calendar"],
                    Some("Ctrl+Alt+M"),
                    "confirm_before_write",
                    &["personal", "memory", "todo"],
                    vec![input(
                        "topic",
                        "Topic",
                        "text",
                        true,
                        "Meeting topic or calendar title",
                    )],
                    vec![
                        step(
                            "personal",
                            "search_timeline",
                            "Find recent related desktop context",
                        ),
                        step("memory", "search", "Recall explicit relevant facts"),
                        step(
                            "todo",
                            "write",
                            "Draft meeting questions and action checklist",
                        ),
                    ],
                ),
                recipe(
                    "project_resume",
                    "Resume project",
                    "Reconstruct project state from workspace, timeline, tasks, and recent actions.",
                    &["project", "resume", "timeline"],
                    Some("Ctrl+Alt+R"),
                    "auto_read_only",
                    &["personal", "flight_recorder", "todo"],
                    vec![input(
                        "project",
                        "Project",
                        "text",
                        true,
                        "Project name or workspace",
                    )],
                    vec![
                        step(
                            "personal",
                            "list_project_workspaces",
                            "Find matching project workspace",
                        ),
                        step(
                            "personal",
                            "search_timeline",
                            "Find recent project activity",
                        ),
                        step(
                            "flight_recorder",
                            "view",
                            "Review recent actions and approvals",
                        ),
                        step("todo", "read", "Read open tasks"),
                    ],
                ),
                recipe(
                    "weekly_report",
                    "Create weekly report",
                    "Build a weekly report from recent work, completed tasks, approvals, and follow-up items.",
                    &["reporting", "summary", "office"],
                    Some("Ctrl+Alt+W"),
                    "confirm_before_write",
                    &["personal", "flight_recorder", "todo", "word"],
                    vec![
                        input("week", "Week", "text", true, "Week or date range"),
                        input(
                            "audience",
                            "Audience",
                            "text",
                            false,
                            "Who will read the report",
                        ),
                    ],
                    vec![
                        step(
                            "personal",
                            "search_timeline",
                            "Collect recent desktop activity",
                        ),
                        step(
                            "flight_recorder",
                            "view",
                            "Collect actions, approvals, and evidence",
                        ),
                        step("todo", "read", "Collect completed and pending tasks"),
                        step("word", "create", "Create the weekly report document"),
                    ],
                ),
            ],
        }
    }

    pub fn all(&self) -> &[Recipe] {
        &self.recipes
    }

    pub fn get(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.id == id)
    }

    pub fn search(&self, search: RecipeSearch) -> Vec<Recipe> {
        let query_terms: Vec<String> = search
            .query
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect();
        let tag_terms: Vec<String> = search.tags.iter().map(|tag| tag.to_lowercase()).collect();
        let limit = search.limit.max(1);

        self.recipes
            .iter()
            .filter(|recipe| {
                tag_terms.is_empty()
                    || tag_terms
                        .iter()
                        .all(|tag| recipe.tags.iter().any(|value| value == tag))
            })
            .filter(|recipe| {
                if query_terms.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {} {}",
                    recipe.id,
                    recipe.title,
                    recipe.description,
                    recipe.tags.join(" ")
                )
                .to_lowercase();
                query_terms.iter().all(|term| haystack.contains(term))
            })
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn dispatch_plan(&self, request: RecipeDispatchRequest) -> Result<RecipeDispatchPlan> {
        let recipe = self
            .get(&request.recipe_id)
            .ok_or_else(|| anyhow!("unknown recipe {}", request.recipe_id))?;
        let values: HashMap<String, String> = request
            .inputs
            .into_iter()
            .map(|input| (input.name, input.value))
            .collect();

        for input in &recipe.inputs {
            if input.required
                && values
                    .get(&input.name)
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
            {
                return Err(anyhow!("missing required input {}", input.name));
            }
        }

        let summary = summarize_recipe_goal(recipe, &values);
        Ok(RecipeDispatchPlan {
            recipe_id: recipe.id.clone(),
            title: recipe.title.clone(),
            summary,
            approval_policy: recipe.approval_policy.clone(),
            required_tools: recipe.required_tools.clone(),
            steps: recipe
                .steps
                .iter()
                .map(|step| RecipeDispatchStep {
                    tool: step.tool.clone(),
                    action: step.action.clone(),
                    description: step.description.clone(),
                    input_summary: Some(summarize_step_input(step, &values)),
                })
                .collect(),
            hotkey: recipe.hotkey.clone(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn recipe(
    id: &str,
    title: &str,
    description: &str,
    tags: &[&str],
    hotkey: Option<&str>,
    approval_policy: &str,
    required_tools: &[&str],
    inputs: Vec<RecipeInputSpec>,
    steps: Vec<RecipeStepTemplate>,
) -> Recipe {
    Recipe {
        id: id.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        tags: tags.iter().map(|value| value.to_string()).collect(),
        hotkey: hotkey.map(ToOwned::to_owned),
        approval_policy: approval_policy.to_string(),
        required_tools: required_tools
            .iter()
            .map(|value| value.to_string())
            .collect(),
        inputs,
        steps,
    }
}

fn input(
    name: &str,
    label: &str,
    kind: &str,
    required: bool,
    description: &str,
) -> RecipeInputSpec {
    RecipeInputSpec {
        name: name.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
        required,
        description: description.to_string(),
    }
}

fn step(tool: &str, action: &str, description: &str) -> RecipeStepTemplate {
    RecipeStepTemplate {
        tool: tool.to_string(),
        action: action.to_string(),
        description: description.to_string(),
    }
}

fn summarize_recipe_goal(recipe: &Recipe, values: &HashMap<String, String>) -> String {
    if let Some(goal) = values.get("goal") {
        return format!("{}: {}", recipe.title, goal);
    }
    if let Some(project) = values.get("project") {
        return format!("{}: {}", recipe.title, project);
    }
    if let Some(topic) = values.get("topic") {
        return format!("{}: {}", recipe.title, topic);
    }
    format!("{} with {} input(s)", recipe.title, values.len())
}

fn summarize_step_input(step: &RecipeStepTemplate, values: &HashMap<String, String>) -> String {
    let primary = values
        .get("goal")
        .or_else(|| values.get("project"))
        .or_else(|| values.get("topic"))
        .or_else(|| values.get("week"))
        .or_else(|| values.get("url"))
        .or_else(|| values.get("folder"));
    match primary {
        Some(value) => format!("{} using {}", step.action, value),
        None => format!("{} with collected recipe inputs", step.action),
    }
}
