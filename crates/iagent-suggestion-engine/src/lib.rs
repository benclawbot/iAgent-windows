use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use iagent_desktop_monitor::{ContextType, WindowContext};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suggestion {
    pub variant_label: String,
    pub text: String,
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<SuggestionIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionCard>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<SuggestionDebug>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionIntent {
    SummarizeEmail,
    DraftReply,
    ExtractTasks,
    PrepareJiraTicket,
    BuildSlideOutline,
    FillForm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionCard {
    pub intent: SuggestionIntent,
    pub confidence: f32,
    pub risk_level: String,
    pub approval_required: bool,
    pub required_inputs: Vec<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuggestionDebug {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_detected: Option<SuggestionIntent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_confidence: Option<f32>,
    pub fallback_to_rewrite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub min_text_length: usize,
    pub max_variants: usize,
    pub context_aware: bool,
    pub cache_ttl_secs: u64,
    pub cache_capacity: usize,
    pub intent_confidence_threshold: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            min_text_length: 50,
            max_variants: 3,
            context_aware: true,
            cache_ttl_secs: 300,
            cache_capacity: 512,
            intent_confidence_threshold: 0.55,
        }
    }
}

#[async_trait]
pub trait LanguageModelProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
struct CachedSuggestions {
    suggestions: Vec<Suggestion>,
    created_at: Instant,
}

pub struct SuggestionEngine {
    provider: Arc<dyn LanguageModelProvider>,
    cache: Arc<RwLock<LruCache<String, CachedSuggestions>>>,
    config: EngineConfig,
}

impl SuggestionEngine {
    pub fn new(provider: Arc<dyn LanguageModelProvider>, config: EngineConfig) -> Self {
        let cap = NonZeroUsize::new(config.cache_capacity.max(1)).expect("non-zero capacity");
        Self {
            provider,
            cache: Arc::new(RwLock::new(LruCache::new(cap))),
            config,
        }
    }

    pub async fn generate_suggestions(
        &self,
        text: &str,
        context: &WindowContext,
    ) -> Result<Vec<Suggestion>> {
        let word_count = text.split_whitespace().count();
        if word_count < self.config.min_text_length {
            return Ok(Vec::new());
        }

        let cache_key = Self::cache_key(text, context);
        if let Some(cached) = self.check_cache(&cache_key).await {
            return Ok(cached);
        }

        let intent_candidate = self.classify_intent(text, context);
        if let Some(intent_suggestion) =
            self.intent_card_suggestion(text, context, &intent_candidate)
        {
            let suggestions = vec![intent_suggestion];
            self.update_cache(cache_key, suggestions.clone()).await;
            return Ok(suggestions);
        }

        let prompt = self.build_prompt(text, context);
        let response = self.provider.complete(&prompt).await?;
        let mut suggestions = self.parse_suggestions(&response)?;
        if let Some((intent, confidence)) = intent_candidate
            && confidence < self.config.intent_confidence_threshold
        {
            let fallback_reason = format!(
                "intent_confidence {:.2} below threshold {:.2}",
                confidence, self.config.intent_confidence_threshold
            );
            for suggestion in &mut suggestions {
                suggestion.debug = Some(SuggestionDebug {
                    intent_detected: Some(intent.clone()),
                    intent_confidence: Some(confidence),
                    fallback_to_rewrite: true,
                    fallback_reason: Some(fallback_reason.clone()),
                });
            }
        }
        self.update_cache(cache_key, suggestions.clone()).await;
        Ok(suggestions)
    }

    async fn check_cache(&self, key: &str) -> Option<Vec<Suggestion>> {
        let mut cache = self.cache.write().await;
        let entry = cache.get(key)?;
        if entry.created_at.elapsed() > Duration::from_secs(self.config.cache_ttl_secs) {
            let _ = cache.pop(key);
            return None;
        }
        Some(entry.suggestions.clone())
    }

    async fn update_cache(&self, key: String, suggestions: Vec<Suggestion>) {
        let mut cache = self.cache.write().await;
        cache.put(
            key,
            CachedSuggestions {
                suggestions,
                created_at: Instant::now(),
            },
        );
    }

    fn cache_key(text: &str, context: &WindowContext) -> String {
        format!(
            "{:?}:{}:{}",
            context.context_type,
            context.app_name,
            text.trim()
        )
    }

    fn build_prompt(&self, text: &str, context: &WindowContext) -> String {
        let context_instruction = if self.config.context_aware {
            match context.context_type {
                ContextType::Email => {
                    "This is an email draft. Vary formality while keeping intent."
                }
                ContextType::Document => {
                    "This is document prose. Improve clarity, structure, and flow."
                }
                ContextType::Presentation => {
                    "This is slide content. Prefer concise, high-impact wording."
                }
                ContextType::Code => "This is code-related text. Keep terms technically precise.",
                ContextType::Chat => "This is chat text. Provide casual and professional variants.",
                ContextType::Browser => {
                    "This is browser-authored text. Improve readability and tone."
                }
                ContextType::Unknown => "Provide alternative phrasings for the text.",
            }
        } else {
            "Provide alternative phrasings for the text."
        };

        format!(
            r#"{context_instruction}

Original text:
"""
{text}
"""

Return JSON only with this schema:
{{
  "suggestions": [
    {{"label": "More Direct", "text": "...", "reasoning": "..."}},
    {{"label": "Diplomatic", "text": "...", "reasoning": "..."}},
    {{"label": "Concise", "text": "...", "reasoning": "..."}}
  ]
}}
"#
        )
    }

    fn intent_card_suggestion(
        &self,
        text: &str,
        context: &WindowContext,
        intent_candidate: &Option<(SuggestionIntent, f32)>,
    ) -> Option<Suggestion> {
        let (intent, confidence) = intent_candidate.clone()?;
        if confidence < self.config.intent_confidence_threshold {
            return None;
        }
        let card = build_action_card(intent.clone(), confidence, text, context);
        Some(Suggestion {
            variant_label: format!("Action: {:?}", intent),
            text: action_title(&intent).to_string(),
            reasoning: Some(format!(
                "Detected intent {:?} with confidence {:.2}.",
                intent, confidence
            )),
            intent: Some(intent.clone()),
            confidence: Some(confidence),
            action: Some(card),
            debug: Some(SuggestionDebug {
                intent_detected: Some(intent),
                intent_confidence: Some(confidence),
                fallback_to_rewrite: false,
                fallback_reason: None,
            }),
        })
    }

    fn classify_intent(
        &self,
        text: &str,
        context: &WindowContext,
    ) -> Option<(SuggestionIntent, f32)> {
        let lowered = text.to_ascii_lowercase();

        let contains_any = |patterns: &[&str]| -> bool {
            patterns.iter().any(|pattern| lowered.contains(pattern))
        };

        if contains_any(&["fill form", "form", "submit", "checkbox"]) {
            return Some((SuggestionIntent::FillForm, 0.78));
        }
        if contains_any(&["jira", "ticket", "issue key", "backlog"]) {
            return Some((SuggestionIntent::PrepareJiraTicket, 0.8));
        }
        if contains_any(&["action items", "tasks", "todo", "next steps"]) {
            return Some((SuggestionIntent::ExtractTasks, 0.72));
        }
        if contains_any(&["slide", "deck", "presentation", "speaker notes"]) {
            return Some((SuggestionIntent::BuildSlideOutline, 0.74));
        }
        if contains_any(&["summarize", "summary", "tl;dr"])
            && context.context_type == ContextType::Email
        {
            return Some((SuggestionIntent::SummarizeEmail, 0.83));
        }
        if contains_any(&["reply", "respond", "response", "follow up"])
            && context.context_type == ContextType::Email
        {
            return Some((SuggestionIntent::DraftReply, 0.82));
        }

        None
    }

    fn parse_suggestions(&self, response: &str) -> Result<Vec<Suggestion>> {
        #[derive(Debug, Deserialize)]
        struct ResponseEnvelope {
            suggestions: Vec<ResponseSuggestion>,
        }

        #[derive(Debug, Deserialize)]
        struct ResponseSuggestion {
            label: String,
            text: String,
            reasoning: Option<String>,
        }

        let parsed: ResponseEnvelope =
            serde_json::from_str(response).context("suggestion response was not valid JSON")?;
        let suggestions = parsed
            .suggestions
            .into_iter()
            .filter(|entry| !entry.text.trim().is_empty())
            .take(self.config.max_variants.max(1))
            .map(|entry| Suggestion {
                variant_label: entry.label,
                text: entry.text,
                reasoning: entry.reasoning,
                intent: None,
                confidence: None,
                action: None,
                debug: None,
            })
            .collect::<Vec<_>>();
        Ok(suggestions)
    }
}

fn action_title(intent: &SuggestionIntent) -> &'static str {
    match intent {
        SuggestionIntent::SummarizeEmail => "Summarize this email",
        SuggestionIntent::DraftReply => "Draft a reply",
        SuggestionIntent::ExtractTasks => "Extract tasks",
        SuggestionIntent::PrepareJiraTicket => "Prepare Jira ticket",
        SuggestionIntent::BuildSlideOutline => "Build slide outline",
        SuggestionIntent::FillForm => "Fill form",
    }
}

fn build_action_card(
    intent: SuggestionIntent,
    confidence: f32,
    text: &str,
    context: &WindowContext,
) -> ActionCard {
    let (risk_level, approval_required, required_inputs) = match intent {
        SuggestionIntent::SummarizeEmail => (
            "read_only".to_string(),
            false,
            vec!["source_text".to_string()],
        ),
        SuggestionIntent::DraftReply => (
            "external_send".to_string(),
            true,
            vec!["recipient".to_string(), "tone".to_string()],
        ),
        SuggestionIntent::ExtractTasks => (
            "read_only".to_string(),
            false,
            vec!["source_text".to_string()],
        ),
        SuggestionIntent::PrepareJiraTicket => (
            "external_send".to_string(),
            true,
            vec!["project_key".to_string(), "assignee".to_string()],
        ),
        SuggestionIntent::BuildSlideOutline => (
            "edit_local".to_string(),
            false,
            vec!["target_file".to_string()],
        ),
        SuggestionIntent::FillForm => (
            "external_send".to_string(),
            true,
            vec!["form_url".to_string(), "field_values".to_string()],
        ),
    };

    ActionCard {
        intent,
        confidence,
        risk_level,
        approval_required,
        required_inputs,
        payload: serde_json::json!({
            "source_app": context.app_name,
            "context_type": format!("{:?}", context.context_type),
            "source_text": text,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockProvider {
        calls: AtomicUsize,
        payload: String,
    }

    #[async_trait]
    impl LanguageModelProvider for MockProvider {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.payload.clone())
        }
    }

    fn sample_context() -> WindowContext {
        WindowContext {
            app_name: "OUTLOOK.EXE".to_string(),
            window_title: "Draft".to_string(),
            context_type: ContextType::Email,
            text_content: None,
            cursor_position: (0, 0),
        }
    }

    #[tokio::test]
    async fn skips_short_inputs() {
        let provider = Arc::new(MockProvider {
            calls: AtomicUsize::new(0),
            payload: r#"{"suggestions":[{"label":"More Direct","text":"x","reasoning":null}]}"#
                .to_string(),
        });
        let engine = SuggestionEngine::new(provider.clone(), EngineConfig::default());
        let out = engine
            .generate_suggestions("too short", &sample_context())
            .await
            .expect("short inputs should not fail");
        assert!(out.is_empty());
        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn high_confidence_intent_bypasses_provider() {
        let provider = Arc::new(MockProvider {
            calls: AtomicUsize::new(0),
            payload: r#"{"suggestions":[{"label":"More Direct","text":"x","reasoning":null}]}"#
                .to_string(),
        });
        let cfg = EngineConfig {
            min_text_length: 1,
            ..EngineConfig::default()
        };
        let engine = SuggestionEngine::new(provider.clone(), cfg);
        let out = engine
            .generate_suggestions(
                "Please draft a reply to this customer email",
                &sample_context(),
            )
            .await
            .expect("intent generation should succeed");

        assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
        assert_eq!(out.len(), 1);
        assert!(out[0].intent.is_some());
        assert!(out[0].action.is_some());
        assert_eq!(
            out[0].debug.as_ref().map(|meta| meta.fallback_to_rewrite),
            Some(false)
        );
    }

    #[tokio::test]
    async fn low_confidence_intent_falls_back_with_debug_metadata() {
        let payload = r#"{
            "suggestions":[
                {"label":"More Direct","text":"A","reasoning":"r1"}
            ]
        }"#;
        let provider = Arc::new(MockProvider {
            calls: AtomicUsize::new(0),
            payload: payload.to_string(),
        });
        let cfg = EngineConfig {
            min_text_length: 1,
            intent_confidence_threshold: 0.9,
            ..EngineConfig::default()
        };
        let engine = SuggestionEngine::new(provider.clone(), cfg);
        let out = engine
            .generate_suggestions(
                "Please draft a reply to this customer email",
                &sample_context(),
            )
            .await
            .expect("rewrite fallback should succeed");

        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(out.len(), 1);
        assert!(out[0].action.is_none());
        let debug = out[0]
            .debug
            .as_ref()
            .expect("low-confidence fallback should include debug metadata");
        assert!(debug.fallback_to_rewrite);
        assert_eq!(debug.intent_detected, Some(SuggestionIntent::DraftReply));
        assert!(
            debug
                .fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("below threshold")
        );
    }

    #[tokio::test]
    async fn caches_provider_responses() {
        let payload = r#"{
            "suggestions":[
                {"label":"More Direct","text":"A","reasoning":"r1"},
                {"label":"Diplomatic","text":"B","reasoning":"r2"},
                {"label":"Concise","text":"C","reasoning":"r3"}
            ]
        }"#;
        let provider = Arc::new(MockProvider {
            calls: AtomicUsize::new(0),
            payload: payload.to_string(),
        });
        let cfg = EngineConfig {
            min_text_length: 1,
            ..EngineConfig::default()
        };
        let engine = SuggestionEngine::new(provider.clone(), cfg);
        let context = sample_context();
        let text = "this input has enough words for deterministic cache testing";

        let first = engine
            .generate_suggestions(text, &context)
            .await
            .expect("first generation should succeed");
        let second = engine
            .generate_suggestions(text, &context)
            .await
            .expect("cached generation should succeed");
        assert_eq!(first, second);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    }
}
