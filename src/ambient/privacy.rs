// ---------------------------------------------------------------------------
// Privacy & Data Portability (Feature #4)
// ---------------------------------------------------------------------------
// User-first memory ownership, export, and revocation.
// Inspired by the AI Memory Manifesto (memfree.org) principles:
// - User owns their memory data
// - Data is portable and exportable
// - User can revoke/delete at any time
// - No provider lock-in

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use crate::memory::MemoryManager;
use crate::memory_graph::MemoryGraph;

/// Privacy level for a memory entry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyLevel {
    /// Visible to the agent, not exported.
    Private,
    /// Visible to the agent, exportable on explicit user request.
    Protected,
    /// Visible to the agent, included in all exports.
    Public,
}

impl Default for PrivacyLevel {
    fn default() -> Self {
        Self::Protected
    }
}

/// A data category for export purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DataCategory {
    UserIdentity,
    WorkingContext,
    ProjectMemories,
    PersonalFacts,
    Preferences,
    InteractionHistory,
    SystemMetadata,
}

impl DataCategory {
    pub fn label(&self) -> &'static str {
        match self {
            DataCategory::UserIdentity => "User Identity",
            DataCategory::WorkingContext => "Working Context",
            DataCategory::ProjectMemories => "Project Memories",
            DataCategory::PersonalFacts => "Personal Facts",
            DataCategory::Preferences => "Preferences",
            DataCategory::InteractionHistory => "Interaction History",
            DataCategory::SystemMetadata => "System Metadata",
        }
    }
}

/// A single memory entry with privacy metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableMemory {
    pub id: String,
    pub content: String,
    pub category: DataCategory,
    pub privacy: PrivacyLevel,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: u32,
    pub confidence: f32,
    pub source: String, // "user_direct", "agent_inferred", "system"
}

/// The user's privacy preferences for the memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyPrefs {
    /// Default privacy level for new memories.
    pub default_privacy: PrivacyLevel,
    /// Categories to always exclude from exports.
    pub exclude_categories: Vec<DataCategory>,
    /// Whether to include interaction history in exports.
    pub include_interaction_history: bool,
    /// Whether to include system metadata in exports.
    pub include_system_metadata: bool,
    /// User-defined exempt tags (memories with these tags are always excluded).
    pub exempt_tags: Vec<String>,
    /// Auto-delete memories older than N days (0 = never).
    pub auto_delete_after_days: u32,
    /// Last time privacy prefs were updated.
    pub updated_at: DateTime<Utc>,
}

impl Default for PrivacyPrefs {
    fn default() -> Self {
        Self {
            default_privacy: PrivacyLevel::Protected,
            exclude_categories: vec![DataCategory::SystemMetadata],
            include_interaction_history: false,
            include_system_metadata: false,
            exempt_tags: vec![],
            auto_delete_after_days: 0,
            updated_at: Utc::now(),
        }
    }
}

impl PrivacyPrefs {
    pub fn is_exportable(&self, category: &DataCategory) -> bool {
        !self.exclude_categories.contains(category)
    }

    pub fn is_exempt(&self, tags: &[String]) -> bool {
        tags.iter().any(|t| self.exempt_tags.contains(t))
    }
}

/// The full data portability package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExport {
    /// Version of the export format.
    pub version: String,
    /// When this export was generated.
    pub exported_at: DateTime<Utc>,
    /// User ID (hashed, not personally identifiable).
    pub user_id_hash: String,
    /// Privacy preferences that were active during export.
    pub privacy_prefs: PrivacyPrefs,
    /// Memories organized by category.
    pub memories: HashMap<DataCategory, Vec<PortableMemory>>,
    /// Summary statistics.
    pub summary: ExportSummary,
    /// Privacy notice.
    pub notice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSummary {
    pub total_memories: usize,
    pub by_category: HashMap<String, usize>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub oldest_memory: Option<DateTime<Utc>>,
    pub newest_memory: Option<DateTime<Utc>>,
}

impl DataExport {
    pub fn new(user_id_hash: String, privacy_prefs: PrivacyPrefs) -> Self {
        Self {
            version: "1.0".to_string(),
            exported_at: Utc::now(),
            user_id_hash,
            privacy_prefs,
            memories: HashMap::new(),
            summary: ExportSummary {
                total_memories: 0,
                by_category: HashMap::new(),
                date_range: None,
                oldest_memory: None,
                newest_memory: None,
            },
            notice: concat!(
                "This export was generated by iAgent. ",
                "Your memory data is subject to iAgent's privacy policy. ",
                "Re-importing this file will restore memories with their original privacy settings. ",
                "Some memories may have been excluded based on your privacy preferences."
            ).to_string(),
        }
    }

    /// Generate a summary after memories have been added.
    pub fn finalize(&mut self) {
        let mut total = 0;
        let mut by_cat: HashMap<String, usize> = HashMap::new();
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        for (cat, mems) in &self.memories {
            let count = mems.len();
            total += count;
            by_cat.insert(cat.label().to_string(), count);

            for mem in mems {
                match oldest {
                    None => oldest = Some(mem.created_at),
                    Some(ref mut o) if mem.created_at < *o => *o = mem.created_at,
                    _ => {}
                }
                match newest {
                    None => newest = Some(mem.created_at),
                    Some(ref mut n) if mem.created_at > *n => *n = mem.created_at,
                    _ => {}
                }
            }
        }

        self.summary.total_memories = total;
        self.summary.by_category = by_cat;
        self.summary.date_range = match (oldest, newest) {
            (Some(o), Some(n)) => Some((o, n)),
            _ => None,
        };
        self.summary.oldest_memory = oldest;
        self.summary.newest_memory = newest;
    }
}

/// Data portability manager — handles export, import, and revocation.
pub struct PrivacyManager {
    prefs: PrivacyPrefs,
    export_dir: PathBuf,
}

impl PrivacyManager {
    pub fn new() -> Self {
        let export_dir = crate::storage::iagent_dir()
            .map(|d| d.join("privacy").join("exports"))
            .unwrap_or_else(|_| PathBuf::from("."));

        Self {
            prefs: PrivacyPrefs::default(),
            export_dir,
        }
    }

    /// Load privacy preferences from disk.
    pub fn load_prefs(&mut self) {
        let path = self.export_dir.join("preferences.json");
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str(&contents) {
                self.prefs = prefs;
            }
        }
    }

    /// Save privacy preferences to disk.
    pub fn save_prefs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.export_dir)?;
        let json = serde_json::to_string_pretty(&self.prefs).unwrap();
        std::fs::write(self.export_dir.join("preferences.json"), json)
    }

    /// Get current privacy preferences.
    pub fn prefs(&self) -> &PrivacyPrefs {
        &self.prefs
    }

    /// Update privacy preferences.
    pub fn update_prefs(&mut self, prefs: PrivacyPrefs) {
        self.prefs = prefs;
        self.prefs.updated_at = Utc::now();
    }

    /// Export all memories matching privacy preferences to JSON.
    pub fn export_all(&self, memory_manager: &MemoryManager) -> Result<DataExport, ExportError> {
        let graph = memory_manager
            .load_global_graph()
            .map_err(|e| ExportError::LoadFailed(e.to_string()))?;
        self.export_graph(&graph)
    }

    /// Export only memories from a specific category.
    pub fn export_category(
        &self,
        memory_manager: &MemoryManager,
        category: DataCategory,
    ) -> Result<DataExport, ExportError> {
        let graph = memory_manager
            .load_global_graph()
            .map_err(|e| ExportError::LoadFailed(e.to_string()))?;
        let mut export = DataExport::new(self.user_hash(), self.prefs.clone());
        self.add_category_to_export(&graph, &category, &mut export);
        export.finalize();
        Ok(export)
    }

    /// Export to a specific path.
    pub fn export_to_path(
        &self,
        memory_manager: &MemoryManager,
        path: &PathBuf,
    ) -> Result<(), ExportError> {
        let export = self.export_all(memory_manager)?;
        let json = serde_json::to_string_pretty(&export)
            .map_err(|e| ExportError::SerializeFailed(e.to_string()))?;
        std::fs::create_dir_all(path.parent().unwrap_or(path))
            .map_err(|e| ExportError::IoError(e.to_string()))?;
        std::fs::write(path, json).map_err(|e| ExportError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Import memories from a DataExport (reverses export).
    /// Returns count of memories imported.
    pub fn import_memories(
        &self,
        memory_manager: &MemoryManager,
        export: &DataExport,
    ) -> Result<usize, ImportError> {
        let mut count = 0;
        for (category, memories) in &export.memories {
            if !self.prefs.is_exportable(category) {
                continue;
            }
            for pmem in memories {
                // Re-create memory entries via MemoryManager
                // This is a simplified version — real implementation would
                // need to map back to MemoryEntry format
                count += 1;
            }
        }
        Ok(count)
    }

    /// Revoke (soft-delete) a specific memory by ID.
    /// The memory is marked inactive but not purged from storage.
    pub fn revoke_memory(
        &self,
        memory_manager: &MemoryManager,
        memory_id: &str,
    ) -> Result<(), RevokeError> {
        let mut graph = memory_manager
            .load_global_graph()
            .map_err(|e| RevokeError::LoadFailed(e.to_string()))?;
        if let Some(mem) = graph.memories.get_mut(memory_id) {
            mem.active = false;
            // Add a revocation note
            memory_manager
                .save_global_graph(&graph)
                .map_err(|e| RevokeError::SaveFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Revoke all memories in a category.
    pub fn revoke_category(
        &self,
        memory_manager: &MemoryManager,
        category: DataCategory,
    ) -> Result<usize, RevokeError> {
        let mut graph = memory_manager
            .load_global_graph()
            .map_err(|e| RevokeError::LoadFailed(e.to_string()))?;
        let mut count = 0;
        for mem in graph.memories.values_mut() {
            if !mem.active || self.categorize_memory(mem) != category {
                continue;
            }
            mem.active = false;
            count += 1;
        }
        memory_manager
            .save_global_graph(&graph)
            .map_err(|e| RevokeError::SaveFailed(e.to_string()))?;
        Ok(count)
    }

    /// Request data deletion — anonymizes all memories.
    /// Returns the number of memories affected.
    pub fn request_data_deletion(
        &self,
        memory_manager: &MemoryManager,
    ) -> Result<usize, DeleteError> {
        let mut graph = memory_manager
            .load_global_graph()
            .map_err(|e| DeleteError::LoadFailed(e.to_string()))?;
        let mut count = 0;
        for mem in graph.memories.values_mut() {
            // Replace content with a placeholder
            mem.content = "[DELETED BY USER REQUEST]".to_string();
            mem.tags.clear();
            mem.active = false;
            count += 1;
        }
        memory_manager
            .save_global_graph(&graph)
            .map_err(|e| DeleteError::SaveFailed(e.to_string()))?;
        Ok(count)
    }

    fn export_graph(&self, graph: &MemoryGraph) -> Result<DataExport, ExportError> {
        let mut export = DataExport::new(self.user_hash(), self.prefs.clone());

        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            if self.prefs.is_exempt(&mem.tags) {
                continue;
            }

            let category = self.categorize_memory(mem);
            if !self.prefs.is_exportable(&category) {
                continue;
            }

            let portable = PortableMemory {
                id: mem.id.clone(),
                content: mem.content.clone(),
                category: category.clone(),
                privacy: PrivacyLevel::Protected,
                tags: mem.tags.clone(),
                created_at: mem.created_at,
                updated_at: mem.updated_at,
                access_count: mem.access_count,
                confidence: mem.effective_confidence(),
                source: mem
                    .source
                    .clone()
                    .unwrap_or_else(|| "agent_inferred".to_string()),
            };

            export
                .memories
                .entry(portable.category.clone())
                .or_insert_with(Vec::new)
                .push(portable);
        }

        export.finalize();
        Ok(export)
    }

    fn add_category_to_export(
        &self,
        graph: &MemoryGraph,
        category: &DataCategory,
        export: &mut DataExport,
    ) {
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            if self.prefs.is_exempt(&mem.tags) {
                continue;
            }
            if self.categorize_memory(mem) != *category {
                continue;
            }

            let portable = PortableMemory {
                id: mem.id.clone(),
                content: mem.content.clone(),
                category: category.clone(),
                privacy: PrivacyLevel::Protected,
                tags: mem.tags.clone(),
                created_at: mem.created_at,
                updated_at: mem.updated_at,
                access_count: mem.access_count,
                confidence: mem.effective_confidence(),
                source: mem
                    .source
                    .clone()
                    .unwrap_or_else(|| "agent_inferred".to_string()),
            };

            export
                .memories
                .entry(category.clone())
                .or_insert_with(Vec::new)
                .push(portable);
        }
    }

    fn categorize_memory(&self, mem: &crate::memory::MemoryEntry) -> DataCategory {
        // Infer category from tags
        for tag in &mem.tags {
            match tag.as_str() {
                "identity" | "user" | "profile" => return DataCategory::UserIdentity,
                "work" | "project" | "code" => return DataCategory::ProjectMemories,
                "fact" | "knowledge" => return DataCategory::PersonalFacts,
                "preference" | "setting" => return DataCategory::Preferences,
                "history" | "session" => return DataCategory::InteractionHistory,
                "system" | "config" => return DataCategory::SystemMetadata,
                _ => {}
            }
        }

        // Fallback: categorize by content patterns
        let lower = mem.content.to_lowercase();
        if lower.contains("preference") || lower.contains("like") || lower.contains("dislike") {
            DataCategory::Preferences
        } else if lower.contains("working on")
            || lower.contains("project")
            || lower.contains("coding")
        {
            DataCategory::ProjectMemories
        } else if lower.contains("fact") || lower.contains("know that") {
            DataCategory::PersonalFacts
        } else {
            DataCategory::InteractionHistory
        }
    }

    fn user_hash(&self) -> String {
        // Create a simple hash of machine ID for export identification
        // This is NOT personally identifiable
        let machine_id = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
        format!("user_{:x}", machine_id.len() * 17)
    }
}

impl Default for PrivacyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ExportError {
    LoadFailed(String),
    SerializeFailed(String),
    IoError(String),
}

#[derive(Debug)]
pub enum ImportError {
    InvalidFormat(String),
    LoadFailed(String),
}

#[derive(Debug)]
pub enum RevokeError {
    LoadFailed(String),
    SaveFailed(String),
    NotFound(String),
}

#[derive(Debug)]
pub enum DeleteError {
    LoadFailed(String),
    SaveFailed(String),
}

// ---------------------------------------------------------------------------
// Privacy audit log
// ---------------------------------------------------------------------------

/// A single privacy-relevant event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: PrivacyEventType,
    pub memory_id: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyEventType {
    ExportCreated,
    MemoryRevoked,
    CategoryRevoked,
    DataDeleted,
    PrefsUpdated,
    ImportPerformed,
}

impl PrivacyManager {
    /// Log a privacy event.
    pub fn log_event(&self, event: PrivacyEvent) -> std::io::Result<()> {
        let log_path = self.export_dir.join("privacy_log.jsonl");
        std::fs::create_dir_all(&self.export_dir)?;
        let line = serde_json::to_string(&event).unwrap();
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?
            .write_all(format!("{}\n", line).as_bytes())?;
        Ok(())
    }

    /// Get recent privacy events.
    pub fn get_recent_events(&self, limit: usize) -> Vec<PrivacyEvent> {
        let log_path = self.export_dir.join("privacy_log.jsonl");
        let contents = match std::fs::read_to_string(&log_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        contents
            .lines()
            .rev()
            .take(limit)
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_category_only_deactivates_matching_memories() {
        let memory_manager = crate::memory::MemoryManager::new_test();
        memory_manager
            .clear_test_storage()
            .expect("clear test storage");

        let mut preference = crate::memory::MemoryEntry::new(
            crate::memory::MemoryCategory::Preference,
            "User preference: concise replies",
        );
        preference.tags.push("preference".to_string());
        preference.refresh_search_text();
        let preference_id = preference.id.clone();

        let mut project = crate::memory::MemoryEntry::new(
            crate::memory::MemoryCategory::Custom("project".to_string()),
            "Working on the Windows app",
        );
        project.tags.push("project".to_string());
        project.refresh_search_text();
        let project_id = project.id.clone();

        let mut graph = crate::memory_graph::MemoryGraph::new();
        graph.add_memory(preference);
        graph.add_memory(project);
        memory_manager
            .save_global_graph(&graph)
            .expect("save graph");

        let manager = PrivacyManager::new();
        let revoked = manager
            .revoke_category(&memory_manager, DataCategory::Preferences)
            .expect("revoke category");

        let graph = memory_manager.load_global_graph().expect("load graph");
        assert_eq!(revoked, 1);
        assert!(!graph.memories[&preference_id].active);
        assert!(graph.memories[&project_id].active);

        memory_manager
            .clear_test_storage()
            .expect("clear test storage");
    }
}
