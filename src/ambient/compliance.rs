// ---------------------------------------------------------------------------
// Ontological Compliance Layer (Feature #5)
// ---------------------------------------------------------------------------
// Evidence-verifiable compliance checking for agent memory claims.
// Inspired by "Ontological Knowledge Blocks" and EVE-Agent's self-verification.
// Ensures the agent's memory statements are consistent with user facts and
// that the agent can verify its own reasoning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use iagent_memory_types::{MemoryEntry, MemoryGraph};

/// A fact claim extracted from memory content.
#[derive(Debug, Clone)]
pub struct FactClaim {
    pub subject: String,   // e.g., "the project", "user"
    pub predicate: String, // e.g., "uses", "prefers", "is"
    pub object: String,    // e.g., "Rust", "dark mode"
    pub source_memory_id: String,
    pub confidence: f32,
}

/// The result of checking a claim against the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// Claim is verified by multiple sources.
    Verified,
    /// Claim has a single source but is plausible.
    Plausible,
    /// Claim contradicts existing verified facts.
    Contradicted {
        conflicting_facts: Vec<String>,
        resolution: String,
    },
    /// Claim cannot be verified due to missing data.
    Unknown,
    /// Claim was verified but is now stale (time-sensitive facts).
    Stale { valid_until: Option<DateTime<Utc>> },
}

impl ClaimStatus {
    pub fn is_compliant(&self) -> bool {
        matches!(self, ClaimStatus::Verified | ClaimStatus::Plausible)
    }
}

/// A compliance rule that must be satisfied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: RuleSeverity,
    /// Tags that this rule applies to. Empty = all memories.
    pub applies_to_tags: Vec<String>,
    /// The condition that must be true.
    pub condition: RuleCondition,
    /// What to do when violated.
    pub action: RuleAction,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RuleSeverity {
    Error,   // Must be satisfied
    Warning, // Should be satisfied
    Info,    // Nice to have
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Claim must have confidence >= threshold.
    MinConfidence(f32),
    /// Fact-category claims must have been verified in the last N days.
    MaxStaleness { category: String, days: u32 },
    /// Claims with this tag must have a source.
    MustHaveSource { tag: String },
    /// Tags that cannot appear together in the same memory.
    MutuallyExclusive { tags: Vec<String> },
    /// Category X memories cannot reference category Y memories.
    NoCrossReferences {
        from_category: String,
        to_category: String,
    },
    /// Content length must be within bounds.
    ContentLengthBounds { min_chars: usize, max_chars: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Reject the memory creation/modification.
    Reject,
    /// Flag as warning but allow.
    Warn,
    /// Require user confirmation.
    RequireConfirmation,
    /// Auto-correct if possible.
    AutoCorrect { correction: String },
}

/// Result of checking one rule against one memory.
#[derive(Debug, Clone)]
pub struct RuleViolation {
    pub rule_id: String,
    pub rule_name: String,
    pub memory_id: String,
    pub severity: RuleSeverity,
    pub message: String,
    pub suggested_fix: Option<String>,
}

/// Full compliance check result for a memory.
#[derive(Debug, Clone)]
pub struct ComplianceResult {
    pub memory_id: String,
    pub status: ComplianceStatus,
    pub violations: Vec<RuleViolation>,
    pub claim_status: ClaimStatus,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ComplianceStatus {
    Compliant,
    Warning,
    Violated,
    Unknown,
}

/// The full compliance engine.
pub struct ComplianceEngine {
    rules: Vec<ComplianceRule>,
    /// Cache of recently verified claims to avoid re-verification.
    claim_cache: HashMap<String, (ClaimStatus, DateTime<Utc>)>,
    /// Map of verified facts: (subject, predicate) -> (object, confidence)
    fact_registry: HashMap<(String, String), VerifiedFact>,
}

#[derive(Debug, Clone)]
pub struct VerifiedFact {
    pub object: String,
    pub confidence: f32,
    pub verified_at: DateTime<Utc>,
    pub source_ids: Vec<String>,
}

impl ComplianceEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            claim_cache: HashMap::new(),
            fact_registry: HashMap::new(),
        };
        engine.load_default_rules();
        engine
    }

    fn load_default_rules(&mut self) {
        self.rules = vec![
            // Fact memories must have reasonable confidence
            ComplianceRule {
                id: "fact_min_confidence".to_string(),
                name: "Fact memories must have high confidence".to_string(),
                description: "Fact-category memories below 0.5 confidence may be unreliable."
                    .to_string(),
                severity: RuleSeverity::Warning,
                applies_to_tags: vec!["fact".to_string()],
                condition: RuleCondition::MinConfidence(0.5),
                action: RuleAction::Warn,
            },
            // Facts must be verified within 30 days
            ComplianceRule {
                id: "fact_max_staleness".to_string(),
                name: "Fact staleness check".to_string(),
                description: "Fact memories not accessed in 30 days may be outdated.".to_string(),
                severity: RuleSeverity::Warning,
                applies_to_tags: vec!["fact".to_string()],
                condition: RuleCondition::MaxStaleness {
                    category: "fact".to_string(),
                    days: 30,
                },
                action: RuleAction::Warn,
            },
            // User preference memories should have a source
            ComplianceRule {
                id: "preference_source".to_string(),
                name: "Preferences must have source".to_string(),
                description:
                    "Preference memories should cite where the preference was inferred from."
                        .to_string(),
                severity: RuleSeverity::Info,
                applies_to_tags: vec!["preference".to_string()],
                condition: RuleCondition::MustHaveSource {
                    tag: "preference".to_string(),
                },
                action: RuleAction::Warn,
            },
            // Memory content should be reasonably sized
            ComplianceRule {
                id: "content_length".to_string(),
                name: "Content length bounds".to_string(),
                description:
                    "Memories with no content or extremely long content may be problematic."
                        .to_string(),
                severity: RuleSeverity::Warning,
                applies_to_tags: vec![],
                condition: RuleCondition::ContentLengthBounds {
                    min_chars: 5,
                    max_chars: 10000,
                },
                action: RuleAction::Warn,
            },
            // Personal facts should not contradict
            ComplianceRule {
                id: "no_self_contradiction".to_string(),
                name: "No self-contradicting facts".to_string(),
                description: "Personal facts should not directly contradict each other."
                    .to_string(),
                severity: RuleSeverity::Error,
                applies_to_tags: vec!["personal".to_string(), "fact".to_string()],
                condition: RuleCondition::MutuallyExclusive {
                    tags: vec!["personal_contradiction".to_string()],
                },
                action: RuleAction::RequireConfirmation,
            },
        ];
    }

    /// Check a single memory against all applicable rules.
    pub fn check_memory(&self, memory: &MemoryEntry) -> ComplianceResult {
        let mut violations = Vec::new();
        let mut status = ComplianceStatus::Compliant;

        for rule in &self.rules {
            if !rule.applies_to_tags.is_empty()
                && !rule.applies_to_tags.iter().any(|t| memory.tags.contains(t))
            {
                continue;
            }

            if let Some(violation) = self.evaluate_rule(rule, memory) {
                if violation.severity == RuleSeverity::Error {
                    status = ComplianceStatus::Violated;
                } else if violation.severity == RuleSeverity::Warning
                    && status != ComplianceStatus::Violated
                {
                    status = ComplianceStatus::Warning;
                }
                violations.push(violation);
            }
        }

        let claim_status = self.verify_claims(memory);

        ComplianceResult {
            memory_id: memory.id.clone(),
            status,
            violations,
            claim_status,
            checked_at: Utc::now(),
        }
    }

    /// Check all memories in a graph and return all violations.
    pub fn check_all(&self, graph: &MemoryGraph) -> Vec<ComplianceResult> {
        graph
            .memories
            .values()
            .filter(|m| m.active)
            .map(|m| self.check_memory(m))
            .collect()
    }

    /// Register a verified fact in the fact registry.
    pub fn register_verified_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        confidence: f32,
        source_ids: Vec<String>,
    ) {
        self.fact_registry.insert(
            (subject.to_string(), predicate.to_string()),
            VerifiedFact {
                object: object.to_string(),
                confidence,
                verified_at: Utc::now(),
                source_ids,
            },
        );
        self.claim_cache.clear(); // Invalidate cache on new facts
    }

    /// Verify a fact claim against the registry.
    pub fn verify_claim(&self, subject: &str, predicate: &str, object: &str) -> ClaimStatus {
        let cache_key = format!("{}:{}:{}", subject, predicate, object);
        if let Some((status, cached_at)) = self.claim_cache.get(&cache_key) {
            // Cache valid for 1 hour
            if (Utc::now() - *cached_at).num_minutes() < 60 {
                return status.clone();
            }
        }

        if let Some(verified) = self
            .fact_registry
            .get(&(subject.to_string(), predicate.to_string()))
        {
            if verified.object == object {
                if (Utc::now() - verified.verified_at).num_days() as i64 > 30 {
                    ClaimStatus::Stale { valid_until: None }
                } else {
                    ClaimStatus::Verified
                }
            } else {
                ClaimStatus::Contradicted {
                    conflicting_facts: vec![format!(
                        "Known: {} {} {}",
                        subject, predicate, verified.object
                    )],
                    resolution: "Facts contradict. Verify which is correct.".to_string(),
                }
            }
        } else {
            ClaimStatus::Unknown
        }
    }

    fn evaluate_rule(&self, rule: &ComplianceRule, memory: &MemoryEntry) -> Option<RuleViolation> {
        match &rule.condition {
            RuleCondition::MinConfidence(threshold) => {
                if memory.effective_confidence() < *threshold {
                    return Some(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        memory_id: memory.id.clone(),
                        severity: rule.severity,
                        message: format!(
                            "Memory confidence {:.0}% is below minimum {:.0}%",
                            memory.effective_confidence() * 100.0,
                            threshold * 100.0
                        ),
                        suggested_fix: Some(format!(
                            "Verify this fact or increase its source confidence"
                        )),
                    });
                }
            }
            RuleCondition::ContentLengthBounds {
                min_chars,
                max_chars,
            } => {
                let len = memory.content.len();
                if len < *min_chars || len > *max_chars {
                    return Some(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        memory_id: memory.id.clone(),
                        severity: rule.severity,
                        message: format!(
                            "Content length {} is outside bounds [{}, {}]",
                            len, min_chars, max_chars
                        ),
                        suggested_fix: if len < *min_chars {
                            Some("Expand this memory with more context".to_string())
                        } else {
                            Some("Consider splitting this into multiple memories".to_string())
                        },
                    });
                }
            }
            RuleCondition::MaxStaleness { category: _, days } => {
                let dur = Utc::now() - memory.updated_at;
                let days_since = dur.num_days();
                if days_since as u32 > *days {
                    return Some(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        memory_id: memory.id.clone(),
                        severity: rule.severity,
                        message: format!(
                            "Memory not updated in {} days (limit: {})",
                            days_since, days
                        ),
                        suggested_fix: Some("Review and refresh this memory".to_string()),
                    });
                }
            }
            _ => {}
        }
        None
    }

    fn verify_claims(&self, memory: &MemoryEntry) -> ClaimStatus {
        // Extract simple claims from content
        let lower = memory.content.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        if words.len() < 3 {
            return ClaimStatus::Unknown;
        }

        // Simple pattern: look for "is", "uses", "prefers", "has"
        let predicates = ["is", "uses", "prefers", "has", "owns", "knows"];
        for (i, word) in words.iter().enumerate() {
            if predicates.contains(word) && i + 2 < words.len() {
                let subject = words[0..i].join(" ");
                let object = words[(i + 1)..].join(" ");
                let claim_key = (subject.clone(), word.to_string());
                if let Some(verified) = self.fact_registry.get(&claim_key) {
                    if verified.object == object {
                        return ClaimStatus::Verified;
                    }
                }
            }
        }

        if self.fact_registry.is_empty() {
            ClaimStatus::Unknown
        } else {
            // If we have a fact registry but no match, it's plausible but unverified
            ClaimStatus::Plausible
        }
    }

    /// Add a custom rule.
    pub fn add_rule(&mut self, rule: ComplianceRule) {
        self.rules.push(rule);
    }

    /// Remove a rule by ID.
    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.retain(|r| r.id != rule_id);
    }

    /// Get all current rules.
    pub fn get_rules(&self) -> &[ComplianceRule] {
        &self.rules
    }

    /// Serialize rules for storage.
    pub fn export_rules(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.rules)
    }

    /// Load rules from JSON.
    pub fn import_rules(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.rules = serde_json::from_str(json)?;
        Ok(())
    }

    /// Generate a compliance summary for the system prompt.
    pub fn compliance_summary(&self, graph: &MemoryGraph) -> String {
        let results = self.check_all(graph);
        let total = results.len();
        let violations = results
            .iter()
            .filter(|r| r.status != ComplianceStatus::Compliant)
            .count();
        let errors = results
            .iter()
            .filter(|r| r.status == ComplianceStatus::Violated)
            .count();

        let mut lines = Vec::new();
        lines.push("## Compliance Status".to_string());
        lines.push(format!(
            "- Checked {} memories: {} warnings, {} violations",
            total,
            violations - errors,
            errors
        ));

        // Top violations
        let top_violations: Vec<_> = results
            .iter()
            .flat_map(|r| r.violations.iter())
            .filter(|v| v.severity == RuleSeverity::Error || v.severity == RuleSeverity::Warning)
            .take(5)
            .collect();

        if !top_violations.is_empty() {
            lines.push("Top issues:".to_string());
            for v in top_violations {
                lines.push(format!(
                    "  - [{}] {}: {}",
                    v.rule_name, v.memory_id, v.message
                ));
            }
        }

        lines.join("\n")
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_claim_returns_registered_fact_status() {
        let mut engine = ComplianceEngine::new();
        engine.register_verified_fact("iagent", "uses", "rust", 0.95, vec!["memory-1".to_string()]);

        assert!(matches!(
            engine.verify_claim("iagent", "uses", "rust"),
            ClaimStatus::Verified
        ));
        assert!(matches!(
            engine.verify_claim("iagent", "uses", "python"),
            ClaimStatus::Contradicted { .. }
        ));
    }
}
