use overlay_ui::{OverlayConfig, run_overlay_daemon};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let headless = std::env::args().any(|arg| arg == "--headless");
    let config = OverlayConfig {
        headless,
        ..OverlayConfig::default()
    };
    run_overlay_daemon(config).await
}
