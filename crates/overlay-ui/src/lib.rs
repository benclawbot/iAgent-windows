use anyhow::Result;
use serde::{Deserialize, Serialize};
use suggestion_engine::Suggestion;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub headless: bool,
    pub hotkey: String,
    pub tray_enabled: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            headless: false,
            hotkey: "Ctrl+Shift+Space".to_string(),
            tray_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportantNotification {
    pub app: String,
    pub title: String,
    pub preview: String,
    pub importance: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlayEvent {
    Suggestions {
        suggestions: Vec<Suggestion>,
        cursor_position: (i32, i32),
    },
    Notification(ImportantNotification),
}

#[derive(Clone)]
pub struct OverlayClient {
    tx: mpsc::UnboundedSender<OverlayEvent>,
}

impl OverlayClient {
    pub fn new(tx: mpsc::UnboundedSender<OverlayEvent>) -> Self {
        Self { tx }
    }

    pub fn show_suggestions(
        &self,
        suggestions: Vec<Suggestion>,
        cursor_position: (i32, i32),
    ) -> Result<()> {
        self.tx.send(OverlayEvent::Suggestions {
            suggestions,
            cursor_position,
        })?;
        Ok(())
    }

    pub fn show_notification(&self, notification: ImportantNotification) -> Result<()> {
        self.tx.send(OverlayEvent::Notification(notification))?;
        Ok(())
    }
}

pub fn spawn_overlay_daemon(config: OverlayConfig) -> (OverlayClient, JoinHandle<()>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<OverlayEvent>();
    let client = OverlayClient::new(tx);
    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if !config.enabled {
                continue;
            }

            match event {
                OverlayEvent::Suggestions {
                    suggestions,
                    cursor_position,
                } => {
                    if config.headless {
                        println!(
                            "[overlay/headless] suggestions={} cursor=({}, {})",
                            suggestions.len(),
                            cursor_position.0,
                            cursor_position.1
                        );
                    }
                }
                OverlayEvent::Notification(notification) => {
                    if config.headless {
                        println!(
                            "[overlay/headless] notification app={} title={} importance={}",
                            notification.app, notification.title, notification.importance
                        );
                    }
                }
            }
        }
    });
    (client, handle)
}

pub async fn run_overlay_daemon(config: OverlayConfig) -> Result<()> {
    let (_client, handle) = spawn_overlay_daemon(config);
    tokio::signal::ctrl_c().await?;
    handle.abort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn client_enqueues_suggestion_events() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let client = OverlayClient::new(tx);
        client
            .show_suggestions(Vec::new(), (1, 2))
            .expect("queue suggestions");
        let event = rx.recv().await.expect("event");
        match event {
            OverlayEvent::Suggestions { cursor_position, .. } => {
                assert_eq!(cursor_position, (1, 2));
            }
            _ => panic!("unexpected event type"),
        }
    }
}
