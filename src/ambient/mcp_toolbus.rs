#![allow(clippy::derivable_impls)]

// ---------------------------------------------------------------------------
// MCP Server / Tool Bus for Ambient (Feature #9)
// ---------------------------------------------------------------------------
// Standardized plugin system for ambient tools, inspired by Percept's
// connectors architecture (Gmail, GitHub, Linear). Replaces the internal-only
// Rust tool approach with a proper plugin bus that can host MCP tools.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A tool definition in the MCP tool bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub auth_required: bool,
    /// Whether this tool is enabled.
    pub enabled: bool,
    /// MCP-style capability tags.
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCategory {
    Communication, // Email, chat, messaging
    Development,   // GitHub, GitLab, code review
    Productivity,  // Calendar, tasks, notes
    Data,          // Database, analytics, API
    System,        // OS-level operations
    Custom,        // User-defined plugins
}

impl ToolCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ToolCategory::Communication => "Communication",
            ToolCategory::Development => "Development",
            ToolCategory::Productivity => "Productivity",
            ToolCategory::Data => "Data",
            ToolCategory::System => "System",
            ToolCategory::Custom => "Custom",
        }
    }
}

/// A registered tool instance with runtime state.
#[derive(Clone)]
pub struct RegisteredTool {
    pub def: ToolDef,
    pub handler: Arc<dyn ToolHandler>,
    /// Connection state (authenticated, token, etc.)
    pub connection: ToolConnection,
    /// Last health check result.
    pub last_health: Option<HealthStatus>,
}

#[derive(Debug, Clone)]
pub enum ToolConnection {
    Disconnected,
    Connected {
        since: chrono::DateTime<chrono::Utc>,
    },
    Error {
        message: String,
    },
}

impl Default for ToolConnection {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// The trait that all tool handlers must implement.
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with given arguments.
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
    /// Check if the tool is healthy and reachable.
    fn health_check(&self) -> Result<HealthStatus, ToolError>;
    /// Get the tool's current connection state.
    fn connection_state(&self) -> ToolConnection;
}

/// Errors from tool execution.
#[derive(Debug)]
pub enum ToolError {
    NotConnected,
    AuthFailed(String),
    ExecutionFailed(String),
    InvalidArgs(String),
    Timeout,
    RateLimited { retry_after_secs: u32 },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::NotConnected => write!(f, "Tool is not connected"),
            ToolError::AuthFailed(s) => write!(f, "Authentication failed: {}", s),
            ToolError::ExecutionFailed(s) => write!(f, "Execution failed: {}", s),
            ToolError::InvalidArgs(s) => write!(f, "Invalid arguments: {}", s),
            ToolError::Timeout => write!(f, "Tool execution timed out"),
            ToolError::RateLimited { retry_after_secs } => {
                write!(f, "Rate limited, retry after {} seconds", retry_after_secs)
            }
        }
    }
}

/// The main tool bus — registry and router for MCP tools.
pub struct ToolBus {
    tools: RwLock<HashMap<String, RegisteredTool>>,
    /// Built-in tools always available without registration.
    builtin_tools: RwLock<Vec<ToolDef>>,
}

impl ToolBus {
    pub fn new() -> Self {
        let bus = Self {
            tools: RwLock::new(HashMap::new()),
            builtin_tools: RwLock::new(Vec::new()),
        };
        bus.register_builtin_tools();
        bus
    }

    fn register_builtin_tools(&self) {
        let builtin = vec![
            ToolDef {
                name: "memory_search".to_string(),
                description: "Search the memory graph for relevant entries".to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "limit": {"type": "integer", "default": 10}
                    },
                    "required": ["query"]
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["search".to_string(), "memory".to_string()],
            },
            ToolDef {
                name: "memory_store".to_string(),
                description: "Store a new memory entry in the graph".to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {"type": "string"},
                        "tags": {"type": "array", "items": {"type": "string"}},
                        "category": {"type": "string"}
                    },
                    "required": ["content"]
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["write".to_string(), "memory".to_string()],
            },
            ToolDef {
                name: "memory_forget".to_string(),
                description: "Mark a memory as inactive (soft delete)".to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {"type": "string"}
                    },
                    "required": ["memory_id"]
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["delete".to_string(), "memory".to_string()],
            },
            ToolDef {
                name: "ambient_status".to_string(),
                description: "Get current ambient mode status and health".to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["read".to_string(), "ambient".to_string()],
            },
            ToolDef {
                name: "initiative_suggest".to_string(),
                description: "Query the initiative engine for current opportunities".to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "default": 5}
                    }
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["read".to_string(), "initiative".to_string()],
            },
            ToolDef {
                name: "compliance_check".to_string(),
                description: "Check a specific memory for compliance violations".to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {"type": "string"}
                    },
                    "required": ["memory_id"]
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["read".to_string(), "compliance".to_string()],
            },
            ToolDef {
                name: "audit_trace".to_string(),
                description: "Trace causal chain for a conclusion back to source memories"
                    .to_string(),
                category: ToolCategory::System,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "conclusion": {"type": "string"},
                        "referenced_ids": {"type": "array", "items": {"type": "string"}}
                    },
                    "required": ["conclusion"]
                }),
                output_schema: None,
                auth_required: false,
                enabled: true,
                capabilities: vec!["read".to_string(), "audit".to_string()],
            },
        ];

        let mut builtins = self.builtin_tools.write().unwrap();
        builtins.extend(builtin);
    }

    /// Register an external MCP tool.
    pub fn register<H: ToolHandler + 'static>(
        &self,
        def: ToolDef,
        handler: Arc<H>,
    ) -> Result<(), RegisterError> {
        let name = def.name.clone();
        let mut tools = self.tools.write().unwrap();
        if tools.contains_key(&name) {
            return Err(RegisterError::AlreadyRegistered(name));
        }
        tools.insert(
            name,
            RegisteredTool {
                def,
                handler,
                connection: ToolConnection::Disconnected,
                last_health: None,
            },
        );
        Ok(())
    }

    /// Unregister a tool.
    pub fn unregister(&self, name: &str) -> Result<(), UnregisterError> {
        let mut tools = self.tools.write().unwrap();
        if tools.remove(name).is_none() {
            return Err(UnregisterError::NotFound(name.to_string()));
        }
        Ok(())
    }

    /// Execute a tool by name.
    pub fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tools = self.tools.read().unwrap();
        let tool = tools.get(name).ok_or(ToolError::NotConnected)?;
        tool.handler.execute(args)
    }

    /// Execute a builtin tool directly.
    pub fn execute_builtin(
        &self,
        name: &str,
        args: serde_json::Value,
        memory_manager: &crate::memory::MemoryManager,
        initiative_engine: Option<&crate::ambient::initiative::InitiativeEngine>,
        compliance_engine: Option<&crate::ambient::compliance::ComplianceEngine>,
        auditor: Option<&crate::ambient::mem_audit::MemoryAuditor>,
    ) -> Result<serde_json::Value, ToolError> {
        match name {
            "memory_search" => self.execute_memory_search(args, memory_manager),
            "memory_store" => self.execute_memory_store(args, memory_manager),
            "memory_forget" => self.execute_memory_forget(args, memory_manager),
            "ambient_status" => self.execute_ambient_status(args),
            "initiative_suggest" => {
                self.execute_initiative_suggest(args, initiative_engine, memory_manager)
            }
            "compliance_check" => {
                self.execute_compliance_check(args, compliance_engine, memory_manager)
            }
            "audit_trace" => self.execute_audit_trace(args, auditor, memory_manager),
            _ => Err(ToolError::NotConnected),
        }
    }

    fn execute_memory_search(
        &self,
        args: serde_json::Value,
        memory_manager: &crate::memory::MemoryManager,
    ) -> Result<serde_json::Value, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'query'".to_string()))?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let results = memory_manager
            .search(query)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let entries: Vec<serde_json::Value> = results
            .into_iter()
            .take(limit)
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "content": m.content,
                    "tags": m.tags,
                    "confidence": m.effective_confidence(),
                })
            })
            .collect();

        Ok(serde_json::json!({ "results": entries }))
    }

    fn execute_memory_store(
        &self,
        args: serde_json::Value,
        memory_manager: &crate::memory::MemoryManager,
    ) -> Result<serde_json::Value, ToolError> {
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'content'".to_string()))?;
        let tags: Vec<String> = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .map(
                |category| match category.parse::<crate::memory::MemoryCategory>() {
                    Ok(category) => category,
                    Err(never) => match never {},
                },
            )
            .unwrap_or_else(|| crate::memory::MemoryCategory::Custom("mcp".to_string()));

        let mut entry = crate::memory::MemoryEntry::new(category, content);
        entry.tags = tags;
        entry.refresh_search_text();

        let id = memory_manager
            .remember_global(entry)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(serde_json::json!({ "id": id, "stored": true }))
    }

    fn execute_memory_forget(
        &self,
        args: serde_json::Value,
        memory_manager: &crate::memory::MemoryManager,
    ) -> Result<serde_json::Value, ToolError> {
        let memory_id = args
            .get("memory_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'memory_id'".to_string()))?;

        // Soft delete via privacy manager would be better, but for now use graph directly
        let mut graph = memory_manager
            .load_global_graph()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        if let Some(mem) = graph.memories.get_mut(memory_id) {
            mem.active = false;
            memory_manager
                .save_global_graph(&graph)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }

        Ok(serde_json::json!({ "forgotten": true, "memory_id": memory_id }))
    }

    fn execute_ambient_status(
        &self,
        _args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        // Return basic ambient status
        Ok(serde_json::json!({
            "mode": "ambient",
            "version": "1.0",
            "features": [
                "graph_relational_context",
                "initiative_engine",
                "user_identity",
                "compliance_layer",
                "memory_auditor",
                "scene_graph",
                "mcp_tool_bus"
            ]
        }))
    }

    fn execute_initiative_suggest(
        &self,
        args: serde_json::Value,
        engine: Option<&crate::ambient::initiative::InitiativeEngine>,
        memory_manager: &crate::memory::MemoryManager,
    ) -> Result<serde_json::Value, ToolError> {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let engine = engine.ok_or_else(|| {
            ToolError::ExecutionFailed("initiative engine not available".to_string())
        })?;

        let candidates = engine.analyze(memory_manager);
        let top: Vec<serde_json::Value> = candidates
            .into_iter()
            .take(limit)
            .map(|c| {
                serde_json::json!({
                    "reason": c.reason,
                    "suggested_actions": c.suggested_actions,
                    "confidence": c.confidence,
                })
            })
            .collect();

        Ok(serde_json::json!({ "candidates": top }))
    }

    fn execute_compliance_check(
        &self,
        args: serde_json::Value,
        engine: Option<&crate::ambient::compliance::ComplianceEngine>,
        memory_manager: &crate::memory::MemoryManager,
    ) -> Result<serde_json::Value, ToolError> {
        let memory_id = args
            .get("memory_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'memory_id'".to_string()))?;
        let engine = engine.ok_or_else(|| {
            ToolError::ExecutionFailed("compliance engine not available".to_string())
        })?;

        let graph = memory_manager
            .load_global_graph()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let mem = graph
            .memories
            .get(memory_id)
            .ok_or_else(|| ToolError::ExecutionFailed("memory not found".to_string()))?;

        let result = engine.check_memory(mem);

        Ok(serde_json::json!({
            "memory_id": result.memory_id,
            "status": format!("{:?}", result.status),
            "violations": result.violations.len(),
            "checked_at": result.checked_at.to_rfc3339(),
        }))
    }

    fn execute_audit_trace(
        &self,
        args: serde_json::Value,
        auditor: Option<&crate::ambient::mem_audit::MemoryAuditor>,
        memory_manager: &crate::memory::MemoryManager,
    ) -> Result<serde_json::Value, ToolError> {
        let conclusion = args
            .get("conclusion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'conclusion'".to_string()))?;
        let referenced_ids: Option<Vec<String>> = args
            .get("referenced_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });
        let auditor = auditor
            .ok_or_else(|| ToolError::ExecutionFailed("auditor not available".to_string()))?;

        let report = auditor.audit(memory_manager, conclusion, referenced_ids.as_deref());

        Ok(serde_json::json!({
            "audited_text": report.audited_text,
            "overall_health": report.overall_health,
            "suspicious_memories": report.suspicious_memories.len(),
            "recommendation": report.recommendation,
            "chains": report.chains.len(),
        }))
    }

    /// List all registered tools (including builtins).
    pub fn list_tools(&self) -> Vec<ToolDef> {
        let mut result = self.builtin_tools.read().unwrap().clone();
        let tools = self.tools.read().unwrap();
        result.extend(tools.values().map(|t| t.def.clone()));
        result
    }

    /// List tools in a specific category.
    pub fn list_by_category(&self, category: ToolCategory) -> Vec<ToolDef> {
        self.list_tools()
            .into_iter()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Get a tool definition by name.
    pub fn get(&self, name: &str) -> Option<ToolDef> {
        // Check builtins first
        if let Some(def) = self
            .builtin_tools
            .read()
            .unwrap()
            .iter()
            .find(|t| t.name == name)
        {
            return Some(def.clone());
        }
        // Then registered
        self.tools.read().unwrap().get(name).map(|t| t.def.clone())
    }

    /// Check health of all registered (non-builtin) tools.
    pub fn health_check_all(&self) -> Vec<(String, HealthStatus)> {
        let tools = self.tools.read().unwrap();
        tools
            .iter()
            .map(|(name, tool)| {
                (
                    name.clone(),
                    tool.handler
                        .health_check()
                        .unwrap_or_else(|_| HealthStatus {
                            ok: false,
                            latency_ms: None,
                            message: Some("unavailable".to_string()),
                            checked_at: chrono::Utc::now(),
                        }),
                )
            })
            .collect()
    }

    /// Get tool execution statistics.
    pub fn stats(&self) -> ToolBusStats {
        let tools = self.tools.read().unwrap();
        ToolBusStats {
            registered_count: tools.len(),
            builtin_count: self.builtin_tools.read().unwrap().len(),
            total_count: tools.len() + self.builtin_tools.read().unwrap().len(),
            connected_count: tools
                .values()
                .filter(|t| matches!(t.connection, ToolConnection::Connected { .. }))
                .count(),
        }
    }

    /// Serialize the tool bus config (for persistence).
    pub fn export_config(&self) -> Result<String, serde_json::Error> {
        let tools = self.tools.read().unwrap();
        let defs: Vec<ToolDef> = tools.values().map(|t| t.def.clone()).collect();
        serde_json::to_string_pretty(&defs)
    }

    /// Load tool bus config from JSON.
    pub fn import_config(&self, json: &str) -> Result<(), serde_json::Error> {
        let _defs: Vec<ToolDef> = serde_json::from_str(json)?;
        // Real implementation would re-register these tools
        Ok(())
    }
}

impl Default for ToolBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_returns_persisted_id_and_applies_tags() {
        let memory_manager = crate::memory::MemoryManager::new_test();
        memory_manager
            .clear_test_storage()
            .expect("clear test storage");
        let bus = ToolBus::new();

        let result = bus
            .execute_builtin(
                "memory_store",
                serde_json::json!({
                    "content": "Remember that ambient bug fixes need tests",
                    "tags": ["ambient", "regression"],
                    "category": "fact"
                }),
                &memory_manager,
                None,
                None,
                None,
            )
            .expect("store memory");

        let id = result["id"].as_str().expect("id").to_string();
        let graph = memory_manager.load_global_graph().expect("load graph");
        let memory = graph.memories.get(&id).expect("stored memory");

        assert_eq!(memory.content, "Remember that ambient bug fixes need tests");
        assert_eq!(memory.category, crate::memory::MemoryCategory::Fact);
        assert_eq!(
            memory.tags,
            vec!["ambient".to_string(), "regression".to_string()]
        );

        memory_manager
            .clear_test_storage()
            .expect("clear test storage");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBusStats {
    pub registered_count: usize,
    pub builtin_count: usize,
    pub total_count: usize,
    pub connected_count: usize,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum RegisterError {
    AlreadyRegistered(String),
    InvalidDefinition(String),
}

#[derive(Debug)]
pub enum UnregisterError {
    NotFound(String),
}

// ---------------------------------------------------------------------------
// Built-in handlers for system tools (no external deps)
// ---------------------------------------------------------------------------

use crate::memory::MemoryManager;

pub struct MemorySearchHandler {
    memory_manager: std::sync::Arc<MemoryManager>,
}

impl MemorySearchHandler {
    pub fn new(memory_manager: std::sync::Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }
}

impl ToolHandler for MemorySearchHandler {
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing 'query'".to_string()))?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let results = self
            .memory_manager
            .search(query)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let results: Vec<_> = results.into_iter().take(limit).collect();

        Ok(serde_json::json!({ "results": results }))
    }

    fn health_check(&self) -> Result<HealthStatus, ToolError> {
        Ok(HealthStatus {
            ok: true,
            latency_ms: Some(0),
            message: None,
            checked_at: chrono::Utc::now(),
        })
    }

    fn connection_state(&self) -> ToolConnection {
        ToolConnection::Connected {
            since: chrono::Utc::now(),
        }
    }
}
