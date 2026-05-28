use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use iagent_desktop_monitor::{
    DesktopMonitor, ImportanceScorer, NotificationDetector, NotificationState, UserPatterns,
};
use iagent_overlay_ui::{
    ImportantNotification as OverlayNotification, OverlayConfig, spawn_overlay_daemon,
};
use iagent_suggestion_engine::{EngineConfig, LanguageModelProvider, SuggestionEngine};

use crate::cli::provider_init::{ProviderChoice, init_provider_and_registry};
use crate::config::config;
use crate::provider::Provider;

struct IAgentProviderAdapter {
    provider: Arc<dyn Provider>,
}

#[async_trait]
impl LanguageModelProvider for IAgentProviderAdapter {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let system = "You generate concise rewrite alternatives for in-progress user writing.";
        self.provider.complete_simple(prompt, system).await
    }
}

fn app_enabled(app_name: &str, enabled: &[String], disabled: &[String]) -> bool {
    let app = app_name.to_ascii_lowercase();
    if disabled
        .iter()
        .any(|d| !d.trim().is_empty() && app.contains(&d.to_ascii_lowercase()))
    {
        return false;
    }
    enabled.is_empty()
        || enabled
            .iter()
            .any(|e| !e.trim().is_empty() && app.contains(&e.to_ascii_lowercase()))
}

pub async fn run(headless: bool) -> Result<()> {
    let ambient_cfg = &config().ambient;
    let monitor = DesktopMonitor::new()?;
    let mut context_rx = monitor.start_monitoring().await;

    let scorer = ImportanceScorer::new(UserPatterns::default());
    let detector = NotificationDetector::new(
        scorer,
        Duration::from_secs(ambient_cfg.desktop_notifications.check_interval_seconds),
    );
    let mut notification_rx = detector.monitor_notifications().await;

    let (provider, _registry) = init_provider_and_registry(&ProviderChoice::Auto, None).await?;
    let adapter = Arc::new(IAgentProviderAdapter { provider });

    let min_text_len = ambient_cfg.desktop_suggestions.min_text_length;
    let engine = SuggestionEngine::new(
        adapter,
        EngineConfig {
            min_text_length: min_text_len,
            cache_ttl_secs: ambient_cfg.desktop_suggestions.cache_ttl_seconds,
            intent_confidence_threshold: ambient_cfg
                .desktop_suggestions
                .intent_confidence_threshold,
            ..EngineConfig::default()
        },
    );

    let overlay_cfg = OverlayConfig {
        headless,
        ..OverlayConfig::default()
    };
    let (overlay_client, _overlay_task) = spawn_overlay_daemon(overlay_cfg);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            Some(context) = context_rx.recv() => {
                if !app_enabled(
                    &context.app_name,
                    &ambient_cfg.desktop_monitoring.enabled_apps,
                    &ambient_cfg.desktop_monitoring.disabled_apps,
                ) {
                    continue;
                }
                if let Some(text) = context.text_content.as_deref() {
                    let suggestions = engine.generate_suggestions(text, &context).await.unwrap_or_default();
                    if !suggestions.is_empty() {
                        crate::core_loop_metrics::record_suggestions_generated(suggestions.len());
                        let _ = overlay_client.show_suggestions(suggestions, context.cursor_position);
                    }
                }
            }
            Some(notification) = notification_rx.recv() => {
                if ambient_cfg.desktop_notifications.enabled
                    && notification.state != NotificationState::Dismissed
                    && notification.importance >= ambient_cfg.desktop_notifications.importance_threshold as f32 {
                    let _ = overlay_client.show_notification(OverlayNotification{
                        app: notification.app,
                        title: notification.title,
                        preview: notification.preview,
                        importance: notification.importance as u8,
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::app_enabled;

    #[test]
    fn app_filter_respects_disabled_first() {
        assert!(!app_enabled(
            "teams.exe",
            &["outlook".to_string(), "teams".to_string()],
            &["teams".to_string()]
        ));
    }

    #[test]
    fn app_filter_accepts_enabled_match() {
        assert!(app_enabled(
            "OUTLOOK.EXE",
            &["outlook".to_string()],
            &Vec::new()
        ));
    }
}
