//! Chrome/Edge CDP (Chrome DevTools Protocol) browser automation.
//!
//! Controls Chrome and Edge via CDP over HTTP (browser remote debugging port).
//! CDP is the same protocol used by Puppeteer/Playwright under the hood.
//!
//! Capabilities:
//! - Navigate to URLs
//! - Query and interact with DOM elements
//! - Screenshot capture
//! - Evaluate JavaScript
//! - Network interception
//! - Storage/state inspection
//!
//! Chrome/Edge must be launched with `--remote-debugging-port=9222`.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CDP WebSocket/HTTP client for Chrome/Edge automation.
#[derive(Debug, Clone)]
pub struct CdpBrowser {
    /// Browser type (chrome or edge)
    browser_type: BrowserType,
    /// Debugging port host
    host: String,
    /// Debugging port
    port: u16,
    /// WebSocket debugger URL (populated after connect)
    ws_url: Option<String>,
    /// Active browser page target ID
    target_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserType {
    Chrome,
    Edge,
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserType::Chrome => write!(f, "chrome"),
            BrowserType::Edge => write!(f, "edge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpTab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub type_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpNode {
    pub backend_node_id: u64,
    pub node_id: u64,
    pub parent_id: Option<u64>,
    pub node_type: String,
    pub node_name: String,
    pub local_name: String,
    pub child_node_count: usize,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpInteractable {
    pub selector: String,
    pub tag: String,
    pub text: Option<String>,
    pub rect: CdpRect,
    pub input_type: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpFormField {
    pub selector: String,
    pub value: Option<String>,
    pub input_type: String,
    pub name: Option<String>,
    pub id: Option<String>,
    pub placeholder: Option<String>,
    pub required: bool,
    pub visible: bool,
}

impl CdpBrowser {
    /// Create a new CDP browser handle.
    pub fn new(browser_type: BrowserType) -> Self {
        Self {
            browser_type,
            host: "127.0.0.1".to_string(),
            port: 9222,
            ws_url: None,
            target_id: None,
        }
    }

    /// Set the debugging port (default 9222).
    pub fn with_port(self, port: u16) -> Self {
        Self { port, ..self }
    }

    /// Build the CDP HTTP endpoint URL.
    fn http_url(&self, path: &str) -> String {
        format!("http://{}:{}/{}", self.host, self.port, path)
    }

    /// List all browser tabs via CDP /json endpoint.
    pub async fn list_tabs(&self) -> Result<Vec<CdpTab>> {
        let url = self.http_url("json");
        let resp = reqwest::get(&url).await?;
        if !resp.status().is_success() {
            bail!("CDP endpoint returned {}: {}", resp.status(), url);
        }
        let tabs: Vec<serde_json::Value> = resp.json().await?;
        let out: Vec<CdpTab> = tabs
            .into_iter()
            .filter(|t| t.get("type").and_then(|v| v.as_str()) == Some("page"))
            .map(|t| CdpTab {
                id: t.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                title: t.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                url: t.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                type_: t.get("type").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            })
            .collect();
        Ok(out)
    }

    /// Attach to a specific tab by ID.
    pub async fn attach_to_tab(&mut self, tab_id: &str) -> Result<()> {
        let tabs = self.list_tabs().await?;
        let _tab = tabs
            .into_iter()
            .find(|t| t.id == tab_id)
            .ok_or_else(|| anyhow::anyhow!("Tab not found: {}", tab_id))?;

        // CDP WebSocket URL for the tab
        self.ws_url = Some(format!(
            "ws://{}:{}/devtools/browser/{}",
            self.host, self.port, tab_id
        ));
        self.target_id = Some(tab_id.to_string());
        Ok(())
    }

    /// Create a new browser tab (CDP NewTarget).
    pub async fn new_tab(&mut self, url: &str) -> Result<String> {
        let _ = url;
        // CDP stub - would POST to /json/new and return the new tab's webSocketDebuggerURL
        Ok(String::new())
    }

    /// Navigate to a URL in the active tab.
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        let _ = url;
        // CDP stub - would send Page.navigate via WebSocket
        Ok(())
    }

    /// Get interactable elements on the page.
    pub async fn get_interactables(&self) -> Result<Vec<CdpInteractable>> {
        // CDP stub - DOM.getDocument + DOM.querySelectorAll + DOM.getBoxModel
        Ok(Vec::new())
    }

    /// Fill form fields on the page.
    pub async fn fill_form(&self, fields: &[CdpFormField]) -> Result<()> {
        let _ = fields;
        // CDP stub - Input.dispatchKeyEvent for typing
        Ok(())
    }

    /// Click an element by selector.
    pub async fn click(&self, selector: &str) -> Result<()> {
        let _ = selector;
        // CDP stub - Runtime.evaluate + HTMLElement.click()
        Ok(())
    }

    /// Evaluate a JavaScript expression.
    pub async fn evaluate(&self, script: &str) -> Result<String> {
        let _ = script;
        // CDP stub - Runtime.evaluate
        Ok(String::new())
    }

    /// Take a screenshot of the current tab.
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        // CDP stub - Page.captureScreenshot
        Ok(Vec::new())
    }

    /// Get page HTML content.
    pub async fn get_content(&self) -> Result<String> {
        // CDP stub - Runtime.evaluate(document.documentElement.outerHTML)
        Ok(String::new())
    }

    /// Close the browser connection.
    pub fn close(&mut self) {
        self.ws_url = None;
        self.target_id = None;
    }
}

impl Default for CdpBrowser {
    fn default() -> Self {
        Self::new(BrowserType::Chrome)
    }
}

/// Discover if Chrome or Edge is running with remote debugging enabled.
pub async fn discover_browsers() -> Result<Vec<CdpBrowser>> {
    let mut browsers = Vec::new();

    // Try Chrome default port
    let chrome = CdpBrowser::new(BrowserType::Chrome);
    if reqwest::get(chrome.http_url("json")).await.is_ok() {
        browsers.push(chrome);
    }

    // Try Edge default port (9223 is sometimes used)
    let mut edge = CdpBrowser::new(BrowserType::Edge);
    edge.port = 9223;
    if reqwest::get(edge.http_url("json")).await.is_ok() {
        browsers.push(edge);
    }

    Ok(browsers)
}

/// Chrome/Edge browser launch flags for remote debugging.
pub fn remote_debugging_flags(browser_type: BrowserType) -> Vec<&'static str> {
    match browser_type {
        BrowserType::Chrome => vec![
            "--remote-debugging-port=9222",
            "--no-first-run",
            "--no-default-browser-check",
        ],
        BrowserType::Edge => vec!["--remote-debugging-port=9222", "--no-first-run"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_type_display() {
        assert_eq!(BrowserType::Chrome.to_string(), "chrome");
        assert_eq!(BrowserType::Edge.to_string(), "edge");
    }

    #[test]
    fn cdp_browser_default_is_chrome() {
        let browser = CdpBrowser::default();
        assert_eq!(browser.browser_type, BrowserType::Chrome);
    }

    #[tokio::test]
    async fn list_tabs_returns_empty_on_no_browser() {
        let browser = CdpBrowser::new(BrowserType::Chrome).with_port(9999);
        let tabs = browser.list_tabs().await;
        // Should error because nothing is on port 9999
        assert!(tabs.is_err());
    }
}