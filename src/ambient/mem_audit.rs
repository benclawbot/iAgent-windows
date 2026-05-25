// ---------------------------------------------------------------------------
// Memory Auditor (Feature #6)
// ---------------------------------------------------------------------------
// Causal attribution to detect when a poisoned memory corrupts downstream
// behavior. When an LLM reaches a wrong conclusion, we trace backwards to
// find which memories contributed to the error.

use chrono::{Duration, Utc};
use std::collections::HashSet;

use crate::memory::MemoryManager;
use crate::memory_graph::{EdgeKind, MemoryGraph};

/// A memory with its attribution chain — how it influenced a conclusion.
#[derive(Debug, Clone)]
pub struct AttributedMemory {
    pub memory_id: String,
    pub content: String,
    pub attribution_score: f32,   // How much this memory contributed
    pub propagation_depth: usize, // How far it propagated through edges
    pub is_fact: bool,
    pub is_stale: bool,
    pub has_contradictions: bool,
}

/// A reconstructed causal chain from evidence to conclusion.
#[derive(Debug, Clone)]
pub struct CausalChain {
    pub conclusion: String,
    pub root_memories: Vec<AttributedMemory>,
    pub total_memories_influenced: usize,
    pub confidence: f32,
    pub warnings: Vec<String>,
}

/// The result of auditing a specific conclusion or piece of reasoning.
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub audited_text: String,
    pub chains: Vec<CausalChain>,
    pub suspicious_memories: Vec<AttributedMemory>,
    pub overall_health: f32, // 0.0 = poisoned, 1.0 = clean
    pub recommendation: String,
}

/// A trail of influence from one memory to another.
#[derive(Debug, Clone)]
pub struct InfluenceTrail {
    pub from_id: String,
    pub to_id: String,
    pub edge_kind: EdgeKind,
    pub confidence: f32,
    pub depth: usize,
}

/// The memory auditor — traces causal chains through the graph.
pub struct MemoryAuditor {
    max_propagation_depth: usize,
}

impl MemoryAuditor {
    pub fn new() -> Self {
        Self {
            max_propagation_depth: 5,
        }
    }

    /// Audit a piece of text or conclusion against the memory graph.
    /// Returns memories that likely influenced the conclusion.
    pub fn audit(
        &self,
        memory_manager: &MemoryManager,
        conclusion: &str,
        referenced_ids: Option<&[String]>,
    ) -> AuditReport {
        let graph = match memory_manager.load_global_graph() {
            Ok(g) => g,
            Err(_) => return self.empty_report(conclusion),
        };

        let mut suspicious = Vec::new();
        let mut chains = Vec::new();

        // Option 1: User provided IDs of memories used in reasoning
        if let Some(ids) = referenced_ids {
            for id in ids {
                if let Some(mem) = graph.memories.get(id) {
                    let attr = self.attribute_memory(&graph, id, conclusion, 0);
                    if attr.attribution_score > 0.5 || attr.is_stale || attr.has_contradictions {
                        suspicious.push(attr);
                    }
                }
            }
        } else {
            // Option 2: Text-based matching — find memories whose content matches keywords
            let keywords = self.extract_keywords(conclusion);
            let candidate_ids = self.find_relevant_memory_ids(&graph, &keywords);

            for id in candidate_ids {
                let attr = self.attribute_memory(&graph, &id, conclusion, 0);
                if attr.attribution_score > 0.4 || attr.is_stale || attr.has_contradictions {
                    suspicious.push(attr.clone());
                }
            }
        }

        // Sort by attribution score
        suspicious.sort_by(|a, b| b.attribution_score.partial_cmp(&a.attribution_score).unwrap());

        // Build causal chains for top suspicious memories
        let top_suspicious: Vec<_> = suspicious.iter().take(5).cloned().collect();
        for attr in &top_suspicious {
            if attr.propagation_depth > 0 || attr.has_contradictions {
                let chain = self.build_chain(&graph, &attr.memory_id, conclusion);
                if !chain.root_memories.is_empty() {
                    chains.push(chain);
                }
            }
        }

        // Overall health
        let health = if suspicious.is_empty() {
            1.0
        } else {
            let avg_score: f32 = suspicious.iter().map(|s| s.attribution_score).sum::<f32>()
                / suspicious.len() as f32;
            let penalty = if suspicious.iter().any(|s| s.has_contradictions) {
                0.3
            } else {
                0.0
            };
            (1.0 - avg_score - penalty).max(0.0)
        };

        let recommendation = if health < 0.5 {
            "High risk: memories used in this conclusion may be unreliable. \
             Verify facts before acting."
                .to_string()
        } else if health < 0.8 {
            "Moderate risk: some memories show signs of staleness or contradiction. \
             Consider verification."
                .to_string()
        } else {
            "Low risk: memories appear sound.".to_string()
        };

        AuditReport {
            audited_text: conclusion.to_string(),
            chains,
            suspicious_memories: suspicious,
            overall_health: health,
            recommendation,
        }
    }

    /// Manually report that a conclusion was WRONG — trace the responsible memories.
    pub fn trace_poison(
        &self,
        memory_manager: &MemoryManager,
        wrong_conclusion: &str,
        referenced_ids: &[String],
    ) -> AuditReport {
        let graph = match memory_manager.load_global_graph() {
            Ok(g) => g,
            Err(_) => return self.empty_report(wrong_conclusion),
        };

        let mut root_memories = Vec::new();

        for id in referenced_ids {
            let attr = self.trace_back(&graph, id, 0);
            root_memories.push(attr);
        }

        root_memories.sort_by(|a, b| b.attribution_score.partial_cmp(&a.attribution_score).unwrap());

        let warnings: Vec<String> = root_memories
            .iter()
            .filter(|m| m.is_stale || m.has_contradictions)
            .map(|m| {
                if m.has_contradictions {
                    format!("Memory '{}' has contradicting edges — may be the poison source", m.memory_id)
                } else {
                    format!("Memory '{}' is stale — may have contributed to the error", m.memory_id)
                }
            })
            .collect();

        let avg_conf = if root_memories.is_empty() {
            0.0
        } else {
            root_memories.iter().map(|m| m.attribution_score).sum::<f32>() / root_memories.len() as f32
        };

        let chain = CausalChain {
            conclusion: wrong_conclusion.to_string(),
            root_memories,
            total_memories_influenced: referenced_ids.len(),
            confidence: avg_conf,
            warnings,
        };

        let suspicious = vec![]; // Already in the chain
        let overall_health = 0.2; // Deliberately wrong conclusion = poisoned

        AuditReport {
            audited_text: wrong_conclusion.to_string(),
            chains: vec![chain],
            suspicious_memories: suspicious,
            overall_health,
            recommendation: "ERROR TRACED: Mark these memories as unreliable and create Correction entries.".to_string(),
        }
    }

    /// Get all memories that were influenced by a given memory.
    pub fn get_downstream_influence(
        &self,
        memory_manager: &MemoryManager,
        memory_id: &str,
    ) -> Vec<InfluenceTrail> {
        let graph = match memory_manager.load_global_graph() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        let mut trails = Vec::new();
        self.collect_downstream(&graph, memory_id, 0, &mut trails);
        trails
    }

    fn collect_downstream(
        &self,
        graph: &MemoryGraph,
        memory_id: &str,
        depth: usize,
        trails: &mut Vec<InfluenceTrail>,
    ) {
        if depth >= self.max_propagation_depth {
            return;
        }

        if let Some(edges) = graph.edges.get(memory_id) {
            for edge in edges {
                trails.push(InfluenceTrail {
                    from_id: memory_id.to_string(),
                    to_id: edge.target.clone(),
                    edge_kind: edge.kind.clone(),
                    confidence: edge.kind.traversal_weight(),
                    depth,
                });
                self.collect_downstream(graph, &edge.target, depth + 1, trails);
            }
        }
    }

    fn trace_back(&self, graph: &MemoryGraph, memory_id: &str, depth: usize) -> AttributedMemory {
        let mem = graph.memories.get(memory_id);

        let has_contradictions = graph
            .edges
            .get(memory_id)
            .map(|e| e.iter().any(|edge| matches!(edge.kind, EdgeKind::Contradicts)))
            .unwrap_or(false);

        let is_stale = mem
            .map(|m| (Utc::now() - m.updated_at).num_days() > 14)
            .unwrap_or(false);

        let attribution_score = if depth == 0 { 1.0 } else { 0.7_f32.powf(depth as f32) };

        AttributedMemory {
            memory_id: memory_id.to_string(),
            content: mem.map(|m| m.content.clone()).unwrap_or_default(),
            attribution_score,
            propagation_depth: depth,
            is_fact: mem.map(|m| matches!(m.category, crate::memory_types::MemoryCategory::Fact)).unwrap_or(false),
            is_stale,
            has_contradictions,
        }
    }

    fn attribute_memory(
        &self,
        graph: &MemoryGraph,
        memory_id: &str,
        conclusion: &str,
        depth: usize,
    ) -> AttributedMemory {
        let mem = graph.memories.get(memory_id);
        let lower_conclusion = conclusion.to_lowercase();

        // Direct text match score
        let content_match = mem.map(|m| {
            let lower_content = m.content.to_lowercase();
            let keyword_hits = self
                .extract_keywords(conclusion)
                .iter()
                .filter(|kw| lower_content.contains(&kw.as_str()))
                .count();
            keyword_hits as f32 / self.extract_keywords(conclusion).len().max(1) as f32
        }).unwrap_or(0.0);

        let has_contradictions = graph
            .edges
            .get(memory_id)
            .map(|e| e.iter().any(|edge| matches!(edge.kind, EdgeKind::Contradicts)))
            .unwrap_or(false);

        let is_stale = mem
            .map(|m| (Utc::now() - m.updated_at).num_days() > 14)
            .unwrap_or(false);

        // Propagation: check inbound edges
        let inbound_score: f32 = graph
            .edges
            .iter()
            .filter(|(_, edges)| edges.iter().any(|e| e.target == memory_id))
            .count() as f32
            * 0.1;

        let depth_penalty = 0.7_f32.powf(depth as f32);
        let attribution_score = (content_match + inbound_score.min(0.3)).max(0.0) * depth_penalty;

        AttributedMemory {
            memory_id: memory_id.to_string(),
            content: mem.map(|m| m.content.clone()).unwrap_or_default(),
            attribution_score,
            propagation_depth: depth,
            is_fact: mem.map(|m| matches!(m.category, crate::memory_types::MemoryCategory::Fact)).unwrap_or(false),
            is_stale,
            has_contradictions,
        }
    }

    fn build_chain(&self, graph: &MemoryGraph, memory_id: &str, conclusion: &str) -> CausalChain {
        let attr = self.attribute_memory(graph, memory_id, conclusion, 0);

        // Collect what this memory was influenced by (inbound edges)
        let inbound_sources: Vec<_> = graph
            .edges
            .iter()
            .filter(|(_, edges)| edges.iter().any(|e| e.target == *memory_id))
            .map(|(src, _)| src.clone())
            .collect();

        let mut root_memories = vec![attr];
        for src_id in inbound_sources.iter().take(3) {
            let upstream = self.attribute_memory(graph, src_id, conclusion, 1);
            root_memories.push(upstream);
        }

        let confidence = root_memories
            .iter()
            .map(|m| m.attribution_score)
            .sum::<f32>()
            / root_memories.len().max(1) as f32;

        let warnings: Vec<String> = root_memories
            .iter()
            .filter(|m| m.is_stale || m.has_contradictions)
            .map(|m| {
                if m.has_contradictions {
                    format!("CONTRADICTION in '{}'", m.memory_id)
                } else {
                    format!("STALE memory '{}'", m.memory_id)
                }
            })
            .collect();

        CausalChain {
            conclusion: conclusion.to_string(),
            root_memories,
            total_memories_influenced: inbound_sources.len(),
            confidence,
            warnings,
        }
    }

    fn find_relevant_memory_ids(&self, graph: &MemoryGraph, keywords: &[String]) -> Vec<String> {
        let mut scores: Vec<(String, f32)> = Vec::new();

        for (id, mem) in &graph.memories {
            if !mem.active {
                continue;
            }
            let lower_content = mem.content.to_lowercase();
            let hits = keywords.iter().filter(|kw| lower_content.contains(&kw.as_str())).count();
            if hits > 0 {
                scores.push((id.clone(), hits as f32));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(10).map(|(id, _)| id).collect()
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
            "be", "have", "has", "had", "do", "does", "did", "will", "would",
            "should", "could", "may", "might", "can", "this", "that", "these",
            "those", "i", "you", "he", "she", "it", "we", "they", "what", "which",
        ].iter().copied().collect();

        text.split_whitespace()
            .map(|w| w.trim().to_lowercase())
            .filter(|w| w.len() > 3 && !stop_words.contains(w.as_str()))
            .collect()
    }

    fn empty_report(&self, conclusion: &str) -> AuditReport {
        AuditReport {
            audited_text: conclusion.to_string(),
            chains: Vec::new(),
            suspicious_memories: Vec::new(),
            overall_health: 1.0,
            recommendation: "No memory graph available to audit.".to_string(),
        }
    }

    /// Health check for the entire memory graph.
    pub fn health_check(&self, memory_manager: &MemoryManager) -> MemoryHealthSummary {
        let graph = match memory_manager.load_global_graph() {
            Ok(g) => g,
            Err(_) => return MemoryHealthSummary::default(),
        };

        let total_memories = graph.memories.len();
        let mut stale_count = 0;
        let mut contradiction_count = 0;
        let mut low_confidence_count = 0;

        let now = Utc::now();
        for mem in graph.memories.values() {
            if !mem.active {
                continue;
            }
            if (now - mem.updated_at).num_days() > 14 {
                stale_count += 1;
            }
            if mem.effective_confidence() < 0.4 {
                low_confidence_count += 1;
            }
        }

        for edges in graph.edges.values() {
            for edge in edges {
                if matches!(edge.kind, EdgeKind::Contradicts) {
                    contradiction_count += 1;
                }
            }
        }

        let score = if total_memories == 0 {
            1.0
        } else {
            let stale_ratio = stale_count as f32 / total_memories as f32;
            let low_conf_ratio = low_confidence_count as f32 / total_memories as f32;
            let contr_ratio = (contradiction_count as f32 / total_memories as f32).min(1.0);
            1.0 - (stale_ratio * 0.3 + low_conf_ratio * 0.4 + contr_ratio * 0.3)
        };

        MemoryHealthSummary {
            total_memories,
            stale_count,
            contradiction_count,
            low_confidence_count,
            overall_score: score.max(0.0),
            summary: if score < 0.5 {
                "Memory graph health is poor — consider running cleanup".to_string()
            } else if score < 0.8 {
                "Memory graph health is moderate — some stale memories exist".to_string()
            } else {
                "Memory graph health is good".to_string()
            },
        }
    }
}

impl Default for MemoryAuditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of overall memory graph health.
#[derive(Debug, Clone)]
pub struct MemoryHealthSummary {
    pub total_memories: usize,
    pub stale_count: usize,
    pub contradiction_count: usize,
    pub low_confidence_count: usize,
    pub overall_score: f32,
    pub summary: String,
}

impl Default for MemoryHealthSummary {
    fn default() -> Self {
        Self {
            total_memories: 0,
            stale_count: 0,
            contradiction_count: 0,
            low_confidence_count: 0,
            overall_score: 1.0,
            summary: "No data".to_string(),
        }
    }
}