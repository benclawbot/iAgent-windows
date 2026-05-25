// ---------------------------------------------------------------------------
// Cross-session Identity (Feature #2)
// ---------------------------------------------------------------------------
// Persistent user model derived from memory graph analysis.
// Feeds into the system prompt as a first-class citizen so the LLM knows
// who it is working with across sessions.

use chrono::{DateTime, Duration, Timelike, Utc};
use std::collections::HashMap;

use crate::memory::MemoryManager;
use crate::memory_graph::MemoryGraph;
use crate::memory_types::MemoryCategory;

/// How the user typically communicates (derived from Preference category analysis).
#[derive(Debug, Clone)]
pub enum CommunicationStyle {
    Formal,
    Casual,
    Technical,
    Mixed,
}

impl std::fmt::Display for CommunicationStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationStyle::Formal => write!(f, "formal"),
            CommunicationStyle::Casual => write!(f, "casual"),
            CommunicationStyle::Technical => write!(f, "technical"),
            CommunicationStyle::Mixed => write!(f, "mixed"),
        }
    }
}

/// An inferred goal detected from repeated memory patterns.
#[derive(Debug, Clone)]
pub struct InferredGoal {
    pub goal_text: String,
    pub confidence: f32,
    pub source_memories: Vec<String>,
    pub inferred_at: DateTime<Utc>,
}

/// The full user identity profile — persistent across sessions.
#[derive(Debug, Clone)]
pub struct UserIdentityProfile {
    /// Inferred working hours (start_hour, end_hour) in UTC — None if not enough data.
    pub working_hours: Option<(u32, u32)>,

    /// How the user typically communicates.
    pub communication_style: CommunicationStyle,

    /// Tags from high-confidence Preference memories — top project interests.
    pub project_preferences: Vec<String>,

    /// Goals inferred from repeated patterns in memory content.
    pub inferred_goals: Vec<InferredGoal>,

    /// Topics that appeared in recent sessions — current active interests.
    pub recent_interests: Vec<String>,

    /// Inferred timezone string (e.g. "America/New_York") — derived from session timing patterns.
    pub timezone: Option<String>,

    /// How frequently the user interacts: "heavy", "moderate", "light".
    pub interaction_frequency: String,

    /// When this profile was last updated.
    pub last_updated: DateTime<Utc>,
}

impl Default for UserIdentityProfile {
    fn default() -> Self {
        Self {
            working_hours: None,
            communication_style: CommunicationStyle::Mixed,
            project_preferences: Vec::new(),
            inferred_goals: Vec::new(),
            recent_interests: Vec::new(),
            timezone: None,
            interaction_frequency: "unknown".to_string(),
            last_updated: Utc::now(),
        }
    }
}

/// Returns true if a memory category is a Preference.
fn is_preference(mem: &crate::memory_types::MemoryEntry) -> bool {
    matches!(mem.category, MemoryCategory::Preference)
}

/// Collect all high-confidence Preference memories from a graph.
fn collect_preference_memories<'a>(graph: &'a MemoryGraph) -> Vec<&'a crate::memory_types::MemoryEntry> {
    graph
        .memories
        .values()
        .filter(|m| m.active && is_preference(m) && m.effective_confidence() >= 0.5)
        .collect()
}

/// Count occurrences of each tag across Preference memories.
fn count_preference_tags<'a>(memories: &[&'a crate::memory_types::MemoryEntry]) -> HashMap<&'a str, usize> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for mem in memories {
        for tag in &mem.tags {
            let t = tag.as_str();
            *counts.entry(t).or_insert(0) += 1;
        }
    }
    counts
}

/// Analyze a graph's session history to infer working hours.
/// Returns (start_hour, end_hour) in UTC if sufficient data exists.
fn infer_working_hours(graph: &MemoryGraph) -> Option<(u32, u32)> {
    // Access timestamps in memories are updated on recall — use those.
    let mut hour_counts = [0u32; 24];
    let mut total = 0usize;

    for mem in graph.memories.values() {
        if !mem.active {
            continue;
        }
        // Use updated_at as a proxy for when the user was active on this topic.
        let hour = mem.updated_at.hour() as u32;
        hour_counts[hour as usize] += 1;
        total += 1;
    }

    if total < 10 {
        return None;
    }

    // Find the 6-hour window with highest activity
    let mut best_start = 0u32;
    let mut best_count = 0u32;
    for start in 0..24 {
        let count: u32 = hour_counts[start as usize..(start as usize + 6)]
            .iter()
            .sum();
        if count > best_count {
            best_count = count;
            best_start = start;
        }
    }

    let end = (best_start + 6) % 24;
    Some((best_start, end))
}

/// Infer communication style from Preference memory content keywords.
fn infer_communication_style(memories: &[&crate::memory_types::MemoryEntry]) -> CommunicationStyle {
    let mut formal_score = 0isize;
    let mut casual_score = 0isize;
    let mut technical_score = 0isize;

    let formal_kw = ["please", "kindly", "would appreciate", "could you", "prefer formal"];
    let casual_kw = ["no worries", "cheers", "ta", "cheers", "just", "tbh", " TBH", "fyi", "FYI"];
    let tech_kw = ["api", "cli", "sdk", "async", "token", "endpoint", "curl", "json", "rust", "python"];

    for mem in memories {
        let lower = mem.content.to_lowercase();
        for kw in &formal_kw {
            if lower.contains(kw) {
                formal_score += 1;
            }
        }
        for kw in &casual_kw {
            if lower.contains(kw) {
                casual_score += 1;
            }
        }
        for kw in &tech_kw {
            if lower.contains(kw) {
                technical_score += 1;
            }
        }
    }

    if technical_score >= formal_score && technical_score >= casual_score {
        CommunicationStyle::Technical
    } else if casual_score >= formal_score {
        CommunicationStyle::Casual
    } else if formal_score > 0 {
        CommunicationStyle::Formal
    } else {
        CommunicationStyle::Mixed
    }
}

/// Extract top N tags from Preference memories.
fn extract_top_tags(memories: &[&crate::memory_types::MemoryEntry], n: usize) -> Vec<String> {
    let counts = count_preference_tags(memories);
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(n).map(|(tag, _)| tag.to_string()).collect()
}

/// Derive recent interests from all high-confidence memories (not just Preferences).
fn derive_recent_interests(graph: &MemoryGraph, limit: usize) -> Vec<String> {
    let mut all_tags: Vec<String> = Vec::new();
    for mem in graph.memories.values() {
        if !mem.active || mem.effective_confidence() < 0.4 {
            continue;
        }
        for tag in &mem.tags {
            all_tags.push(tag.clone());
        }
    }
    // Count and sort
    let mut tag_counts: HashMap<&str, usize> = HashMap::new();
    for tag in &all_tags {
        *tag_counts.entry(tag.as_str()).or_insert(0) += 1;
    }
    let mut sorted: Vec<_> = tag_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(limit).map(|(t, _)| t.to_string()).collect()
}

/// Estimate interaction frequency from session count per day.
fn estimate_interaction_frequency(graph: &MemoryGraph) -> String {
    // Use memory access patterns: count distinct days with accesses in last 7 days
    let now = Utc::now();
    let seven_days_ago = now - Duration::days(7);

    let mut days_with_activity = std::collections::HashSet::new();
    for mem in graph.memories.values() {
        if !mem.active {
            continue;
        }
        if mem.updated_at >= seven_days_ago {
            days_with_activity.insert(mem.updated_at.date_naive());
        }
    }

    let days_count = days_with_activity.len();
    if days_count >= 6 {
        "heavy".to_string()
    } else if days_count >= 3 {
        "moderate".to_string()
    } else if days_count >= 1 {
        "light".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Infer a timezone rough estimate from hour distribution.
/// If user is most active say between 14-22 UTC, they might be in CET (UTC+1).
/// This is a heuristic — not a real timezone lookup.
fn infer_timezone_heuristic(graph: &MemoryGraph) -> Option<String> {
    let mut hour_counts = [0u32; 24];
    let mut total = 0usize;
    for mem in graph.memories.values() {
        if !mem.active {
            continue;
        }
        hour_counts[mem.updated_at.hour() as usize] += 1;
        total += 1;
    }
    if total < 20 {
        return None;
    }

    // Find peak hour
    let peak_hour = hour_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| *c)
        .map(|(h, _)| h as u32)?;

    // Heuristic: if peak is afternoon UTC (14-18), user likely in Europe/Africa (UTC+1 to UTC+3)
    if peak_hour >= 13 && peak_hour <= 18 {
        Some("推测: Europe/Africa (UTC+1 to +3)".to_string())
    } else if peak_hour >= 20 || peak_hour <= 4 {
        // Night owl — Americas
        Some("推测: Americas (UTC-5 to UTC-8)".to_string())
    } else if peak_hour >= 9 && peak_hour <= 14 {
        Some("推测: Asia/Pacific (UTC+5 to UTC+9)".to_string())
    } else {
        None
    }
}

/// Build a UserIdentityProfile by analyzing the memory graphs.
pub fn build_identity_profile(memory_manager: &MemoryManager) -> UserIdentityProfile {
    let mut profile = UserIdentityProfile::default();

    // Analyze project graph (primary)
    let project_graph = match memory_manager.load_project_graph() {
        Ok(g) => Some(g),
        Err(_) => None,
    };

    // Analyze global graph (secondary)
    let global_graph = match memory_manager.load_global_graph() {
        Ok(g) => Some(g),
        Err(_) => None,
    };

    // --- Preferences from project graph ---
    if let Some(ref graph) = project_graph {
        let prefs = collect_preference_memories(graph);

        // Communication style
        profile.communication_style = infer_communication_style(&prefs);

        // Top project preference tags
        profile.project_preferences = extract_top_tags(&prefs, 8);

        // Working hours
        profile.working_hours = infer_working_hours(graph);

        // Interaction frequency
        profile.interaction_frequency = estimate_interaction_frequency(graph);

        // Timezone heuristic
        profile.timezone = infer_timezone_heuristic(graph);

        // Recent interests from all high-conf memories
        profile.recent_interests = derive_recent_interests(graph, 10);
    }

    // --- Goals: detect repeated patterns in memory content ---
    if let Some(ref graph) = project_graph {
        let mut goal_candidates: HashMap<String, Vec<String>> = HashMap::new();
        for mem in graph.memories.values() {
            if !mem.active || mem.effective_confidence() < 0.5 {
                continue;
            }
            // Look for goal-like phrases: "want to", "need to", "working on", "trying to"
            let lower = mem.content.to_lowercase();
            let goal_indicators = ["want to", "need to", "working on", "trying to", "goal:", "aim:"];
            for indicator in &goal_indicators {
                if lower.contains(indicator) {
                    // Truncate to first 80 chars as the goal text
                    let goal_text = if mem.content.len() > 80 {
                        format!("{}...", &mem.content[..80])
                    } else {
                        mem.content.clone()
                    };
                    goal_candidates
                        .entry(goal_text)
                        .or_insert_with(Vec::new)
                        .push(mem.id.clone());
                }
            }
        }

        for (goal_text, source_ids) in goal_candidates {
            if source_ids.len() >= 2 {
                profile.inferred_goals.push(InferredGoal {
                    goal_text,
                    confidence: 0.6,
                    source_memories: source_ids,
                    inferred_at: Utc::now(),
                });
            }
        }

        // Keep only top 5 goals by source memory count
        profile
            .inferred_goals
            .sort_by(|a, b| b.source_memories.len().cmp(&a.source_memories.len()));
        profile.inferred_goals.truncate(5);
    }

    // Also analyze global graph for cross-project patterns
    if let Some(ref graph) = global_graph {
        let prefs = collect_preference_memories(graph);
        let global_tags = extract_top_tags(&prefs, 3);
        // Merge any new tags not already in project preferences
        for tag in global_tags {
            if !profile.project_preferences.contains(&tag) {
                if profile.project_preferences.len() < 12 {
                    profile.project_preferences.push(tag);
                }
            }
        }
    }

    profile.last_updated = Utc::now();
    profile
}

/// Render the identity as a section for the system prompt.
pub fn build_identity_prompt_section(identity: &UserIdentityProfile) -> String {
    let mut lines = Vec::new();
    lines.push("## User Model (across sessions)".to_string());

    // Working hours
    if let Some((start, end)) = identity.working_hours {
        lines.push(format!(
            "- Working hours: {}:00 - {}:00 UTC",
            start, end
        ));
    }

    // Communication style
    lines.push(format!(
        "- Communication style: {}",
        identity.communication_style
    ));

    // Interaction frequency
    if identity.interaction_frequency != "unknown" {
        lines.push(format!(
            "- Interaction frequency: {} user",
            identity.interaction_frequency
        ));
    }

    // Project preferences
    if !identity.project_preferences.is_empty() {
        lines.push(format!(
            "- Project interests: {}",
            identity.project_preferences.join(", ")
        ));
    }

    // Inferred goals
    if !identity.inferred_goals.is_empty() {
        lines.push("Inferred ongoing goals:".to_string());
        for goal in identity.inferred_goals.iter().take(3) {
            lines.push(format!(
                "  - {} (seen in {} memories)",
                goal.goal_text, goal.source_memories.len()
            ));
        }
    }

    // Recent interests
    if !identity.recent_interests.is_empty() {
        lines.push(format!(
            "- Recent interests: {}",
            identity.recent_interests.join(", ")
        ));
    }

    // Timezone heuristic (if available)
    if let Some(ref tz) = identity.timezone {
        lines.push(format!("- Timezone hint: {}", tz));
    }

    let result = lines.join("\n");
    if result.lines().count() <= 1 {
        String::new()
    } else {
        result
    }
}
