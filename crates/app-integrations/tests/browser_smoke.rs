use app_integrations::browser::{BrowserType, CdpBrowser};
use std::time::Duration;

async fn run_browser_smoke(browser_type: BrowserType, port: u16) -> anyhow::Result<()> {
    let mut browser = CdpBrowser::new(browser_type)
        .with_port(port)
        .with_action_timeout(Duration::from_secs(15))
        .with_max_retries(2);

    let url = std::env::var("IAGENT_BROWSER_SMOKE_URL")
        .unwrap_or_else(|_| "https://example.com".to_string());

    browser.navigate(&url).await?;
    let interactables = browser.get_interactables().await?;
    assert!(
        !interactables.is_empty(),
        "expected interactables for {}",
        url
    );

    // Example.com includes a visible anchor with text containing "more information".
    browser.click("text=more information").await?;

    let screenshot = browser.screenshot().await?;
    assert!(!screenshot.is_empty(), "expected screenshot bytes");

    let html = browser.get_content().await?;
    assert!(html.to_ascii_lowercase().contains("<html"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires Chrome launched with --remote-debugging-port=9222"]
async fn chrome_smoke() {
    let port = std::env::var("IAGENT_CHROME_CDP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(9222);
    run_browser_smoke(BrowserType::Chrome, port)
        .await
        .expect("chrome smoke should pass");
}

#[tokio::test]
#[ignore = "requires Edge launched with --remote-debugging-port=9223"]
async fn edge_smoke() {
    let port = std::env::var("IAGENT_EDGE_CDP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(9223);
    run_browser_smoke(BrowserType::Edge, port)
        .await
        .expect("edge smoke should pass");
}
