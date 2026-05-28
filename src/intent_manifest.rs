use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct IntentManifestStore {
    path: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IntentManifestState {
    #[serde(default)]
    pub manifests: Vec<ImportedIntentManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentManifest {
    pub schema_version: u32,
    pub app_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub actions: Vec<IntentAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentAction {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<IntentParameter>,
    #[serde(default)]
    pub examples: Vec<IntentExample>,
    pub approval_level: String,
    pub rollback_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentParameter {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentExample {
    pub summary: String,
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedIntentManifest {
    pub manifest_path: String,
    pub imported_at: DateTime<Utc>,
    pub app_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub actions: Vec<ImportedIntentAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportedIntentAction {
    pub app_id: String,
    pub app_name: String,
    pub action_id: String,
    pub title: String,
    pub description: String,
    pub approval_level: String,
    pub rollback_hint: String,
    #[serde(default)]
    pub parameters: Vec<IntentParameter>,
    #[serde(default)]
    pub examples: Vec<IntentExample>,
    pub source_manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentActionPlanRequest {
    pub app_id: String,
    pub action_id: String,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentActionPlan {
    pub app_id: String,
    pub app_name: String,
    pub action_id: String,
    pub title: String,
    pub tool: String,
    pub approval_level: String,
    pub rollback_hint: String,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub required_parameters: Vec<String>,
    #[serde(default)]
    pub examples: Vec<IntentExample>,
    #[serde(default)]
    pub steps: Vec<IntentPlanStep>,
    pub source_manifest_path: String,
    pub recipe_import_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentPlanStep {
    pub tool: String,
    pub action: String,
    pub description: String,
}

impl IntentManifestStore {
    pub fn load() -> Result<Self> {
        let dir = crate::storage::iagent_dir()?.join("intent_manifests");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("imported.json"),
        })
    }

    pub fn state(&self) -> Result<IntentManifestState> {
        if self.path.exists() {
            crate::storage::read_json(&self.path)
                .with_context(|| format!("read intent manifests at {}", self.path.display()))
        } else {
            Ok(IntentManifestState::default())
        }
    }

    fn save_state(&self, state: &IntentManifestState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::storage::write_json(&self.path, state)
            .with_context(|| format!("write intent manifests at {}", self.path.display()))
    }

    pub fn read_manifest(path: &Path) -> Result<IntentManifest> {
        let manifest: IntentManifest = crate::storage::read_json(path)
            .with_context(|| format!("read intent manifest at {}", path.display()))?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }

    pub fn discover(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        discover_inner(root, max_depth, &mut found)?;
        found.sort();
        Ok(found)
    }

    pub fn import_manifest(&self, path: &Path) -> Result<ImportedIntentManifest> {
        let manifest = Self::read_manifest(path)?;
        let source_manifest_path = path.to_string_lossy().to_string();
        let imported = ImportedIntentManifest {
            manifest_path: source_manifest_path.clone(),
            imported_at: Utc::now(),
            app_id: manifest.app_id.clone(),
            name: manifest.name.clone(),
            description: manifest.description,
            entrypoint: manifest.entrypoint,
            actions: manifest
                .actions
                .into_iter()
                .map(|action| ImportedIntentAction {
                    app_id: manifest.app_id.clone(),
                    app_name: manifest.name.clone(),
                    action_id: action.id,
                    title: action.title,
                    description: action.description,
                    approval_level: action.approval_level,
                    rollback_hint: action.rollback_hint,
                    parameters: action.parameters,
                    examples: action.examples,
                    source_manifest_path: source_manifest_path.clone(),
                })
                .collect(),
        };

        let mut state = self.state()?;
        state.manifests.retain(|existing| {
            existing.app_id != imported.app_id && existing.manifest_path != imported.manifest_path
        });
        state.manifests.insert(0, imported.clone());
        self.save_state(&state)?;
        Ok(imported)
    }

    pub fn get_manifest(&self, app_id: &str) -> Result<Option<ImportedIntentManifest>> {
        Ok(self
            .state()?
            .manifests
            .into_iter()
            .find(|manifest| manifest.app_id == app_id))
    }

    pub fn list_actions(
        &self,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ImportedIntentAction>> {
        let query = query.unwrap_or_default().to_ascii_lowercase();
        let mut actions: Vec<ImportedIntentAction> = self
            .state()?
            .manifests
            .into_iter()
            .flat_map(|manifest| manifest.actions)
            .filter(|action| {
                if query.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {} {} {}",
                    action.app_id,
                    action.app_name,
                    action.action_id,
                    action.title,
                    action.description
                )
                .to_ascii_lowercase();
                haystack.contains(&query)
            })
            .collect();
        actions.sort_by(|a, b| a.app_name.cmp(&b.app_name).then(a.title.cmp(&b.title)));
        actions.truncate(limit.max(1));
        Ok(actions)
    }

    pub fn plan_action(&self, request: IntentActionPlanRequest) -> Result<IntentActionPlan> {
        let manifest = self
            .get_manifest(&request.app_id)?
            .ok_or_else(|| anyhow!("unknown intent app {}", request.app_id))?;
        let action = manifest
            .actions
            .iter()
            .find(|action| action.action_id == request.action_id)
            .ok_or_else(|| {
                anyhow!(
                    "intent app {} does not expose action {}",
                    request.app_id,
                    request.action_id
                )
            })?;
        validate_plan_parameters(action, &request.parameters)?;
        let required_parameters = action
            .parameters
            .iter()
            .filter(|parameter| parameter.required)
            .map(|parameter| parameter.name.clone())
            .collect();
        Ok(IntentActionPlan {
            app_id: action.app_id.clone(),
            app_name: action.app_name.clone(),
            action_id: action.action_id.clone(),
            title: action.title.clone(),
            tool: "intent".to_string(),
            approval_level: action.approval_level.clone(),
            rollback_hint: action.rollback_hint.clone(),
            parameters: request.parameters,
            required_parameters,
            examples: action.examples.clone(),
            steps: vec![
                IntentPlanStep {
                    tool: "intent".to_string(),
                    action: "plan".to_string(),
                    description: "Validate structured parameters and approval level.".to_string(),
                },
                IntentPlanStep {
                    tool: "flight_recorder".to_string(),
                    action: "view".to_string(),
                    description: "Attach run evidence before and after execution.".to_string(),
                },
            ],
            source_manifest_path: action.source_manifest_path.clone(),
            recipe_import_hint: format!(
                "Use app_id '{}' and action_id '{}' as a typed recipe step after approval.",
                action.app_id, action.action_id
            ),
        })
    }
}

fn validate_manifest(manifest: &IntentManifest) -> Result<()> {
    if manifest.schema_version != 1 {
        return Err(anyhow!(
            "unsupported iagent.intent.json schema_version {}",
            manifest.schema_version
        ));
    }
    require_ident("app_id", &manifest.app_id)?;
    require_text("name", &manifest.name)?;
    if manifest.actions.is_empty() {
        return Err(anyhow!("intent manifest must declare at least one action"));
    }
    let mut ids = std::collections::HashSet::new();
    for action in &manifest.actions {
        require_ident("action id", &action.id)?;
        if !ids.insert(action.id.as_str()) {
            return Err(anyhow!("duplicate action id {}", action.id));
        }
        require_text("action title", &action.title)?;
        require_text("action description", &action.description)?;
        require_text("approval_level", &action.approval_level)?;
        require_text("rollback_hint", &action.rollback_hint)?;
        let mut parameter_names = std::collections::HashSet::new();
        for parameter in &action.parameters {
            require_ident("parameter name", &parameter.name)?;
            require_text("parameter kind", &parameter.kind)?;
            require_text("parameter description", &parameter.description)?;
            if !parameter_names.insert(parameter.name.as_str()) {
                return Err(anyhow!("duplicate parameter {}", parameter.name));
            }
        }
    }
    Ok(())
}

fn validate_plan_parameters(action: &ImportedIntentAction, parameters: &Value) -> Result<()> {
    let Some(object) = parameters.as_object() else {
        return Err(anyhow!("parameters must be an object"));
    };
    for parameter in &action.parameters {
        if parameter.required && !object.contains_key(&parameter.name) {
            return Err(anyhow!("missing required parameter {}", parameter.name));
        }
    }
    Ok(())
}

fn require_ident(label: &str, value: &str) -> Result<()> {
    require_text(label, value)?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(anyhow!(
            "{} may only contain ASCII letters, numbers, dots, underscores, or hyphens",
            label
        ));
    }
    Ok(())
}

fn require_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{} is required", label));
    }
    Ok(())
}

fn discover_inner(root: &Path, depth_remaining: usize, found: &mut Vec<PathBuf>) -> Result<()> {
    if depth_remaining == 0 || !root.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("iagent.intent.json") {
            found.push(path);
            continue;
        }
        if path.is_dir() {
            discover_inner(&path, depth_remaining - 1, found)?;
        }
    }
    Ok(())
}
