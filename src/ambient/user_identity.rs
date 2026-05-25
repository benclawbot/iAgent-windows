// ---------------------------------------------------------------------------
// Cross-Session User Identity Model (Feature #2)
// ---------------------------------------------------------------------------
// Persistent user model that accumulates preferences, working patterns,
// and communication style across sessions. Feeds into the system prompt
// as a first-class citizen.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


use crate::memory::MemoryManager;

/// The user's communication style preference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommunicationStyle {
    Concise,        // Short, to-the-point responses
    Detailed,       // Thorough explanations with context
    Technical,     // Uses jargon, assumes domain knowledge
    Conversational,// Casual, friendly tone
    Formal,        // Professional, structured
}

impl Default for CommunicationStyle {
    fn default() -> Self {
        Self::Conversational
    }
}

/// A working pattern — when and how the user prefers to work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingPattern {
    /// Typical start of work day (hour, 0-23).
    pub typical_start_hour: Option<u32>,
    /// Typical end of work day (hour, 0-23).
    pub typical_end_hour: Option<u32>,
    /// Preferred days for deep work.
    pub deep_work_days: Vec<u32>, // 0=Monday, 6=Sunday
    /// Whether the user is typically active on weekends.
    pub weekend_active: bool,
    /// Preferred response speed for non-urgent matters.
    pub response_speed: ResponseSpeed,
}

impl Default for WorkingPattern {
    fn default() -> Self {
        Self {
            typical_start_hour: None,
            typical_end_hour: None,
            deep_work_days: Vec::new(),
            weekend_active: false,
            response_speed: ResponseSpeed::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ResponseSpeed {
    Immediate, // Minutes
    Normal,    // Hours
    Relaxed,   // Days
}

impl Default for ResponseSpeed {
    fn default() -> Self {
        Self::Normal
    }
}

/// Domain expertise level per topic area.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseLevel {
    /// Self-reported or observed expertise in a topic.
    pub level: Expertise,
    /// Topic/tag this applies to.
    pub topic: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Expertise {
    Novice,
    Intermediate,
    Expert,
}

impl Default for Expertise {
    fn default() -> Self {
        Self::Intermediate
    }
}

/// A specific preference inferred from behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredPreference {
    pub key: String,           // e.g., "reply_length", "tone"
    pub value: String,         // e.g., "concise", "friendly"
    pub confidence: f32,      // 0.0-1.0
    pub source_memory_id: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl InferredPreference {
    pub fn new(key: &str, value: &str, confidence: f32, source_memory_id: Option<String>) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            confidence,
            source_memory_id,
            updated_at: Utc::now(),
        }
    }
}

/// Project context — what the user is currently working on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentProject {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub last_updated: DateTime<Utc>,
    pub is_active: bool,
}

impl CurrentProject {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            tags: Vec::new(),
            last_updated: Utc::now(),
            is_active: true,
        }
    }
}

/// The full cross-session user identity model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique identifier for this user (hashed machine ID).
    pub user_id: String,
    /// Human-readable name or alias.
    pub display_name: Option<String>,
    /// Communication style preference.
    pub communication_style: CommunicationStyle,
    /// Working pattern.
    pub working_pattern: WorkingPattern,
    /// Per-topic expertise levels.
    pub expertise: Vec<ExpertiseLevel>,
    /// Inferred preferences from behavior.
    pub preferences: Vec<InferredPreference>,
    /// Currently active projects.
    pub active_projects: Vec<CurrentProject>,
    /// Last time this identity was updated.
    pub last_updated: DateTime<Utc>,
    /// Accumulated session count.
    pub session_count: u32,
    /// Total hours of interaction.
    pub total_hours: f32,
}

impl UserIdentity {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            display_name: None,
            communication_style: CommunicationStyle::default(),
            working_pattern: WorkingPattern::default(),
            expertise: Vec::new(),
            preferences: Vec::new(),
            active_projects: Vec::new(),
            last_updated: Utc::now(),
            session_count: 0,
            total_hours: 0.0,
        }
    }

    /// Update the identity from recent memory entries.
    pub fn refresh_from_memories(&mut self, memory_manager: &MemoryManager) {
        let graph = match memory_manager.load_global_graph() {
            Ok(g) => g,
            Err(_) => return,
        };

        // Extract communication preferences from memory content
        self.update_communication_style(&graph);
        // Extract working patterns from session metadata
        self.update_working_pattern(&graph);
        // Update expertise levels
        self.update_expertise(&graph);
        // Update inferred preferences
        self.update_preferences(&graph);

        self.last_updated = Utc::now();
    }

    fn update_communication_style(&mut self, graph: &crate::memory_graph::MemoryGraph) {
        // Look for explicit style preferences in memories
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            let lower = mem.content.to_lowercase();
            if lower.contains("be concise") || lower.contains("short responses") {
                self.merge_preference("communication_style", "concise", 0.8, Some(&mem.id));
            } else if lower.contains("be detailed") || lower.contains("explain thoroughly") {
                self.merge_preference("communication_style", "detailed", 0.8, Some(&mem.id));
            } else if lower.contains("technical") && lower.contains("preference") {
                self.merge_preference("communication_style", "technical", 0.7, Some(&mem.id));
            }
        }
    }

    fn update_working_pattern(&mut self, graph: &crate::memory_graph::MemoryGraph) {
        // Extract from session metadata or explicit memories
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            let lower = mem.content.to_lowercase();
            if lower.contains("working hours:") || lower.contains("work day:") {
                // Parse simple patterns like "working hours: 9-5" or "work day: 8am-6pm"
                if let Some(start) = self.extract_hour(&lower, "start") {
                    self.working_pattern.typical_start_hour = Some(start);
                }
                if let Some(end) = self.extract_hour(&lower, "end") {
                    self.working_pattern.typical_end_hour = Some(end);
                }
            }
            if lower.contains("deep work:") || lower.contains("focus day:") {
                // Parse day preferences
                for (day_idx, day_name) in ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
                    .iter().enumerate() {
                    if lower.contains(day_name) {
                        if !self.working_pattern.deep_work_days.contains(&(day_idx as u32)) {
                            self.working_pattern.deep_work_days.push(day_idx as u32);
                        }
                    }
                }
            }
        }
    }

    fn extract_hour(&self, text: &str, _which: &str) -> Option<u32> {
        // Simple pattern: look for numbers followed by am/pm or just hour numbers
        // e.g., "9am", "9:00", "9"
        let re = regex::Regex::new(r"(\d{1,2})(?:[:.](\d{2}))?\s*(am|pm)?").ok()?;
        for cap in re.captures_iter(text) {
            let hour: u32 = cap.get(1).and_then(|m| m.as_str().parse().ok())?;
            let minute: u32 = cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let ampm = cap.get(3).map(|m| m.as_str());

            let hour = if let Some(ampm) = ampm {
                match ampm {
                    "pm" if hour < 12 => hour + 12,
                    "am" if hour == 12 => 0,
                    _ => hour,
                }
            } else {
                hour
            };
            return Some(hour % 24);
        }
        None
    }

    fn update_expertise(&mut self, graph: &crate::memory_graph::MemoryGraph) {
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            let lower = mem.content.to_lowercase();

            for tag in &mem.tags {
                let tag_lower = tag.to_lowercase();
                if lower.contains("expert in") || lower.contains("精通") || lower.contains("specialist") {
                    if let Some(existing) = self.expertise.iter_mut().find(|e| e.topic == *tag) {
                        existing.level = Expertise::Expert;
                    } else {
                        self.expertise.push(ExpertiseLevel {
                            level: Expertise::Expert,
                            topic: tag.clone(),
                        });
                    }
                }
            }
        }
    }

    fn update_preferences(&mut self, graph: &crate::memory_graph::MemoryGraph) {
        // Collect inferred preferences from memory tags and clusters
        let mut new_prefs: Vec<InferredPreference> = Vec::new();

        for mem in graph.memories.values() {
            if !mem.active || mem.effective_confidence() < 0.5 {
                continue;
            }
            let lower = mem.content.to_lowercase();

            // Check for preference indicators
            let key_values = [
                ("reply_length", "concise", "be concise"),
                ("reply_length", "detailed", "explain everything"),
                ("tone", "friendly", "casual"),
                ("tone", "formal", "formal"),
                ("proactive", "yes", "take initiative"),
                ("proactive", "no", "wait for me to ask"),
            ];

            for (key, value, indicator) in &key_values {
                if lower.contains(indicator) {
                    new_prefs.push(InferredPreference::new(key, value, mem.effective_confidence(), Some(mem.id.clone())));
                }
            }
        }

        // Merge, keeping the highest confidence for each key
        for new_pref in new_prefs {
            if let Some(existing) = self.preferences.iter_mut().find(|p| p.key == new_pref.key) {
                if new_pref.confidence > existing.confidence {
                    existing.value = new_pref.value;
                    existing.confidence = new_pref.confidence;
                    existing.source_memory_id = new_pref.source_memory_id;
                    existing.updated_at = Utc::now();
                }
            } else {
                self.preferences.push(new_pref);
            }
        }
    }

    fn merge_preference(&mut self, key: &str, value: &str, confidence: f32, source: Option<&str>) {
        if let Some(existing) = self.preferences.iter_mut().find(|p| p.key == key) {
            if confidence > existing.confidence {
                existing.value = value.to_string();
                existing.confidence = confidence;
                existing.source_memory_id = source.map(String::from);
                existing.updated_at = Utc::now();
            }
        } else {
            self.preferences.push(InferredPreference::new(
                key,
                value,
                confidence,
                source.map(String::from),
            ));
        }
    }

    /// Get a preference value by key, or None if not found.
    pub fn get_preference(&self, key: &str) -> Option<&InferredPreference> {
        self.preferences.iter().find(|p| p.key == key)
    }

    /// Increment session count and add hours.
    pub fn record_session(&mut self, hours: f32) {
        self.session_count += 1;
        self.total_hours += hours;
        self.last_updated = Utc::now();
    }

    /// Serialize to JSON for storage.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for UserIdentity {
    fn default() -> Self {
        Self::new("default".to_string())
    }
}

/// Render the user identity as a section for the system prompt.
pub fn build_identity_section(identity: &UserIdentity) -> String {
    let mut lines = Vec::new();
    lines.push("## User Identity & Preferences".to_string());
    lines.push("(Learned from memory graph across sessions)".to_string());
    lines.push(String::new());

    // Communication style
    lines.push(format!(
        "- Communication style: {}",
        match identity.communication_style {
            CommunicationStyle::Concise => "concise",
            CommunicationStyle::Detailed => "detailed",
            CommunicationStyle::Technical => "technical",
            CommunicationStyle::Conversational => "conversational",
            CommunicationStyle::Formal => "formal",
        }
    ));

    // Working hours
    if let (Some(start), Some(end)) = (identity.working_pattern.typical_start_hour, identity.working_pattern.typical_end_hour) {
        lines.push(format!("- Typical work hours: {}:00 - {}:00", start, end));
    }

    // Deep work days
    if !identity.working_pattern.deep_work_days.is_empty() {
        let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let days: Vec<&str> = identity.working_pattern.deep_work_days
            .iter()
            .filter_map(|d| day_names.get(*d as usize).copied())
            .collect();
        lines.push(format!("- Deep work days: {}", days.join(", ")));
    }

    // Active projects
    let active: Vec<&str> = identity.active_projects.iter()
        .filter(|p| p.is_active)
        .map(|p| p.name.as_str())
        .collect();
    if !active.is_empty() {
        lines.push(format!("- Active projects: {}", active.join(", ")));
    }

    // Key preferences
    for pref in &identity.preferences {
        if pref.confidence > 0.5 {
            lines.push(format!("- Preference ({}% confident): {} = {}",
                (pref.confidence * 100.0) as u32, pref.key, pref.value));
        }
    }

    // Expertise
    let experts: Vec<&str> = identity.expertise.iter()
        .filter(|e| e.level == Expertise::Expert)
        .map(|e| e.topic.as_str())
        .collect();
    if !experts.is_empty() {
        lines.push(format!("- Expert in: {}", experts.join(", ")));
    }

    lines.push(String::new());
    lines.push(format!(
        "(Session #{}{})",
        identity.session_count,
        if identity.total_hours > 0.0 {
            format!(" — {:.1} total hours", identity.total_hours)
        } else {
            String::new()
        }
    ));

    lines.join("\n")
}
