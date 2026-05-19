use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use desktop_monitor::{ContextType, WindowContext};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Suggestion {
    pub variant_label: String,
    pub text: String,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub min_text_length: usize,
    pub max_variants: usize,
    pub context_aware: bool,
    pub cache_ttl_secs: u64,
    pub cache_capacity: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            min_text_length: 50,
            max_variants: 3,
            context_aware: true,
            cache_ttl_secs: 300,
            cache_capacity: 512,
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

        let prompt = self.build_prompt(text, context);
        let response = self.provider.complete(&prompt).await?;
        let suggestions = self.parse_suggestions(&response)?;
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
            context.context_type, context.app_name, text.trim()
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
                ContextType::Code => {
                    "This is code-related text. Keep terms technically precise."
                }
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
            })
            .collect::<Vec<_>>();
        Ok(suggestions)
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

