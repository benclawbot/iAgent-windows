// ---------------------------------------------------------------------------
// Initiative Engine (Feature #3)
// ---------------------------------------------------------------------------
// Detects patterns in the memory graph and surfaces proactive opportunities
// to the LLM. Replaces pure clock-driven scheduling with goal-driven reasoning.

use chrono::Utc;
use std::collections::HashMap;

use crate::ambient::Priority;
use crate::memory::MemoryManager;
use crate::memory_graph::MemoryGraph;
use crate::memory_types::MemoryCategory;

/// A trigger that caused an initiative candidate to be generated.
#[derive(Debug, Clone)]
pub enum InitiativeTrigger {
    /// Repeated concept across multiple memories — user is actively interested.
    MemoryPattern {
        description: String,
        priority: Priority,
        suggestion: String,
    },
    /// A Fact-category memory not accessed recently — may be stale.
    StaleFact {
        memory_id: String,
        content_snippet: String,
        days_since_access: u32,
    },
    /// User keeps returning to correct the same topic.
    RepeatedCorrection {
        memory_id: String,
        content_snippet: String,
        correction_count: usize,
    },
    /// An opportunity detected from high-confidence memories in an idle cluster.
    OpportunityDetected {
        description: String,
        estimated_value: String,
        cluster_id: String,
    },
    /// Progress toward a known inferred goal.
    GoalProgress {
        goal_text: String,
        progress_pct: f32,
        related_memories: Vec<String>,
    },
}

/// A candidate initiative to surface to the LLM.
#[derive(Debug, Clone)]
pub struct InitiativeCandidate {
    pub trigger: InitiativeTrigger,
    pub reason: String,
    pub suggested_actions: Vec<String>,
    pub confidence: f32,
}

impl InitiativeCandidate {
    fn new(
        trigger: InitiativeTrigger,
        reason: &str,
        suggested_actions: Vec<&str>,
        confidence: f32,
    ) -> Self {
        Self {
            trigger,
            reason: reason.to_string(),
            suggested_actions: suggested_actions.into_iter().map(String::from).collect(),
            confidence,
        }
    }
}

/// The initiative analysis engine.
#[derive(Debug)]
pub struct InitiativeEngine {}

impl InitiativeEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze both project and global graphs to produce initiative candidates.
    pub fn analyze(&self, memory_manager: &MemoryManager) -> Vec<InitiativeCandidate> {
        let mut candidates = Vec::new();

        let project_graph = memory_manager.load_project_graph().ok();
        let global_graph = memory_manager.load_global_graph().ok();

        if let Some(ref graph) = project_graph {
            let graph_candidates = self.analyze_graph(graph);
            candidates.extend(graph_candidates);
        }

        if let Some(ref graph) = global_graph {
            let global_candidates = self.analyze_graph(graph);
            // Only keep global candidates that don't overlap with project ones
            for gc in global_candidates {
                let overlapping = candidates.iter().any(|c| match (&gc.trigger, &c.trigger) {
                    (
                        InitiativeTrigger::MemoryPattern { description: a, .. },
                        InitiativeTrigger::MemoryPattern { description: b, .. },
                    ) => a == b,
                    _ => false,
                });
                if !overlapping {
                    candidates.push(gc);
                }
            }
        }

        // Sort by confidence descending and keep top 8
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        candidates.truncate(8);
        candidates
    }

    fn analyze_graph(&self, graph: &MemoryGraph) -> Vec<InitiativeCandidate> {
        let mut candidates = Vec::new();
        let now = Utc::now();

        // --- 1. Repeated tag patterns: active interests ---
        let tag_counts = self.count_active_tags(graph);
        let repeated_tags: Vec<_> = tag_counts
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .collect();
        for (tag, count) in repeated_tags {
            let mems_with_tag: Vec<_> = graph
                .memories
                .values()
                .filter(|m| m.active && m.tags.iter().any(|t| t.as_str() == tag))
                .collect();

            if mems_with_tag.len() >= 3 {
                candidates.push(InitiativeCandidate::new(
                    InitiativeTrigger::MemoryPattern {
                        description: format!("Topic '{}' appears in {} memories", tag, count),
                        priority: Priority::Normal,
                        suggestion: format!(
                            "Dig deeper into '{}': summarize what you know, identify gaps, \
                             and consider proactively updating relevant memories.",
                            tag
                        ),
                    },
                    &format!(
                        "'{}' has been mentioned {} times — this is an active interest area.",
                        tag, count
                    ),
                    vec![
                        &format!("Review existing memories about '{}'", tag),
                        "Identify contradictions or stale facts in this cluster",
                        "Consider if the user needs an update or summary on this topic",
                    ],
                    0.75,
                ));
            }
        }

        // --- 2. Stale Facts ---
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            if !matches!(mem.category, MemoryCategory::Fact) {
                continue;
            }
            // Consider a fact stale if not accessed in 14 days
            let days_since = (now - mem.updated_at).num_days().max(0) as u32;
            if days_since > 14 {
                let snippet = if mem.content.len() > 55 {
                    format!("{}...", &mem.content[..55])
                } else {
                    mem.content.clone()
                };
                candidates.push(InitiativeCandidate::new(
                    InitiativeTrigger::StaleFact {
                        memory_id: mem.id.clone(),
                        content_snippet: snippet.clone(),
                        days_since_access: days_since,
                    },
                    &format!(
                        "Fact '{}' has not been accessed in {} days — may need verification.",
                        snippet, days_since
                    ),
                    vec![
                        "Verify this fact is still accurate",
                        "Update the memory with current information if needed",
                        "If incorrect, create a Correction memory",
                    ],
                    0.6,
                ));
            }
        }

        // --- 3. Repeated corrections ---
        // A memory with Correction category linked to the same target multiple times
        let mut correction_counts: HashMap<String, usize> = HashMap::new();
        for (src_id, edges) in &graph.edges {
            for edge in edges {
                if matches!(edge.kind, crate::memory_graph::EdgeKind::Contradicts) {
                    *correction_counts.entry(src_id.clone()).or_insert(0) += 1;
                }
            }
        }
        for (mem_id, count) in correction_counts {
            if count >= 2
                && let Some(mem) = graph.memories.get(&mem_id)
            {
                let snippet = if mem.content.len() > 50 {
                    format!("{}...", &mem.content[..50])
                } else {
                    mem.content.clone()
                };
                candidates.push(InitiativeCandidate::new(
                    InitiativeTrigger::RepeatedCorrection {
                        memory_id: mem_id.clone(),
                        content_snippet: snippet.clone(),
                        correction_count: count,
                    },
                    &format!(
                        "'{}' has been contradicted {} times — the agent may keep making the same mistake.",
                        snippet, count
                    ),
                    vec![
                        "Flag this as a known error pattern",
                        "Create a strong Correction memory with evidence",
                        "In next cycle, check before reasoning about this topic",
                    ],
                    0.8,
                ));
            }
        }

        // --- 4. Opportunities: high-confidence memories in neglected clusters ---
        // A cluster is neglected if its memories haven't been accessed recently
        for (cluster_id, cluster) in &graph.clusters {
            if cluster.name.is_none() {
                continue;
            }
            let cluster_name = cluster.name.as_ref().unwrap();

            // Find memories in this cluster via InCluster edges
            let member_ids: Vec<String> = graph
                .edges
                .iter()
                .filter(|(_src, edges)| {
                    edges.iter().any(|e| {
                        e.target == *cluster_id
                            && matches!(e.kind, crate::memory_graph::EdgeKind::InCluster)
                    })
                })
                .map(|(src, _)| src.clone())
                .collect();

            if member_ids.is_empty() {
                continue;
            }

            let members: Vec<_> = member_ids
                .iter()
                .filter_map(|id| graph.memories.get(id))
                .filter(|m| m.active)
                .collect();

            if members.is_empty() {
                continue;
            }

            // Check if any member was accessed recently
            let recent_access =
                members
                    .iter()
                    .map(|m| m.updated_at)
                    .fold(None, |acc, ts| match acc {
                        None => Some(ts),
                        Some(prev) if ts > prev => Some(ts),
                        other => other,
                    });

            let days_since = recent_access.map_or(999, |ts| (now - ts).num_days() as u32);

            if days_since > 21 {
                // High-confidence members but no recent access — opportunity
                let avg_conf = {
                    let sum: f32 = members.iter().map(|m| m.effective_confidence()).sum();
                    sum / members.len() as f32
                };
                if avg_conf > 0.5 {
                    candidates.push(InitiativeCandidate::new(
                        InitiativeTrigger::OpportunityDetected {
                            description: format!(
                                "Cluster '{}' has {} high-confidence memories not accessed in {} days",
                                cluster_name, members.len(), days_since
                            ),
                            estimated_value: format!("avg confidence {:.0}%", avg_conf * 100.0),
                            cluster_id: cluster_id.clone(),
                        },
                        &format!(
                            "The '{}' cluster has valuable memories that haven't been reviewed recently.",
                            cluster_name
                        ),
                        vec![
                            &format!("Review cluster '{}' and extract key insights", cluster_name),
                            "Update stale information in this cluster",
                            "Consider connecting these memories to current work",
                        ],
                        0.55,
                    ));
                }
            }
        }

        // --- 5. Inferred goals from repeated goal-like phrases ---
        let mut goal_memories: HashMap<String, Vec<String>> = HashMap::new();
        for mem in graph.memories.values() {
            if !mem.active || mem.effective_confidence() < 0.4 {
                continue;
            }
            let lower = mem.content.to_lowercase();
            if lower.contains("goal:") || lower.contains("aim:") || lower.contains("working toward")
            {
                let key = if mem.content.len() > 80 {
                    format!("{}...", &mem.content[..80])
                } else {
                    mem.content.clone()
                };
                goal_memories.entry(key).or_default().push(mem.id.clone());
            }
        }
        for (goal_text, memory_ids) in goal_memories {
            if memory_ids.len() >= 2 {
                let related_count = memory_ids.len();
                candidates.push(InitiativeCandidate::new(
                    InitiativeTrigger::GoalProgress {
                        goal_text: goal_text.clone(),
                        progress_pct: 0.5, // Placeholder — would need subtask tracking to be accurate
                        related_memories: memory_ids,
                    },
                    &format!(
                        "Goal '{}' appears in {} memories — appears to be ongoing work.",
                        goal_text, related_count
                    ),
                    vec![
                        "Check progress on this goal",
                        "Update or complete relevant memories",
                        "Consider suggesting next action to the user",
                    ],
                    0.65,
                ));
            }
        }

        candidates
    }

    fn count_active_tags(&self, graph: &MemoryGraph) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            for tag in &mem.tags {
                *counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        counts
    }
}

impl Default for InitiativeEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Render initiative candidates as a section for the system prompt.
pub fn build_initiative_section(candidates: &[InitiativeCandidate]) -> String {
    if candidates.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    lines.push("## Proactive Opportunities".to_string());
    lines.push("(Detected from memory graph analysis — act on the most relevant)".to_string());
    lines.push(String::new());

    for (i, candidate) in candidates.iter().enumerate().take(8) {
        lines.push(format!(
            "### Opportunity {} (confidence: {:.0}%)\n{}",
            i + 1,
            candidate.confidence * 100.0,
            candidate.reason
        ));

        // Suggested actions
        if !candidate.suggested_actions.is_empty() {
            lines.push("Suggested actions:".to_string());
            for action in &candidate.suggested_actions {
                lines.push(format!("  - {}", action));
            }
        }

        lines.push(String::new());
    }

    lines.join("\n")
}
