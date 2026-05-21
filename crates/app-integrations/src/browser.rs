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
//! - Storage/state inspection
//!
//! Chrome/Edge must be launched with `--remote-debugging-port=9222`.

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DEFAULT_ACTION_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_MAX_RETRIES: usize = 2;
const RETRY_BACKOFF_MS: u64 = 120;

/// CDP browser wrapper for Chrome/Edge DevTools Protocol automation
#[allow(dead_code)]
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
    /// Timeout applied to each browser action.
    action_timeout: Duration,
    /// Retry count for transient failures.
    max_retries: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserType {
    Chrome,
    Edge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserErrorCategory {
    Transient,
    InvalidInput,
    TargetMissing,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowserErrorInfo {
    pub category: BrowserErrorCategory,
    pub message: String,
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
    pub ws_url: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
struct RawCdpTab {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default, rename = "type")]
    type_: String,
    #[serde(default, rename = "webSocketDebuggerUrl")]
    ws_url: String,
}

impl RawCdpTab {
    fn to_tab(&self) -> Option<CdpTab> {
        if self.type_ != "page" {
            return None;
        }
        Some(CdpTab {
            id: self.id.clone(),
            title: self.title.clone(),
            url: self.url.clone(),
            type_: self.type_.clone(),
            ws_url: if self.ws_url.is_empty() {
                None
            } else {
                Some(self.ws_url.clone())
            },
        })
    }
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
            action_timeout: DEFAULT_ACTION_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }

    /// Set the debugging port (default 9222).
    pub fn with_port(self, port: u16) -> Self {
        Self { port, ..self }
    }

    /// Set a per-action timeout.
    pub fn with_action_timeout(self, timeout: Duration) -> Self {
        Self {
            action_timeout: timeout,
            ..self
        }
    }

    /// Set retry count for transient failures.
    pub fn with_max_retries(self, max_retries: usize) -> Self {
        Self {
            max_retries,
            ..self
        }
    }

    fn classify_error_message(message: &str) -> BrowserErrorCategory {
        let msg = message.to_ascii_lowercase();
        if msg.contains("timeout")
            || msg.contains("timed out")
            || msg.contains("temporar")
            || msg.contains("connection reset")
            || msg.contains("broken pipe")
            || msg.contains("connection refused")
            || msg.contains("dns")
            || msg.contains("channel closed")
            || msg.contains("websocket closed")
        {
            return BrowserErrorCategory::Transient;
        }

        if msg.contains("invalid selector")
            || msg.contains("syntaxerror")
            || msg.contains("runtime.evaluate exception")
            || msg.contains("failed serializing")
            || msg.contains("failed parsing")
        {
            return BrowserErrorCategory::InvalidInput;
        }

        if msg.contains("tab not found")
            || msg.contains("no page targets")
            || msg.contains("no debuggable page target")
            || msg.contains("missing websocket")
            || msg.contains("not found")
            || msg.contains("no clickable element")
            || msg.contains("not visible")
            || msg.contains("no data field")
        {
            return BrowserErrorCategory::TargetMissing;
        }

        BrowserErrorCategory::Fatal
    }

    fn normalize_error(message: impl Into<String>) -> anyhow::Error {
        let message = message.into();
        let category = Self::classify_error_message(&message);
        anyhow!(
            "[browser/{category:?}] {}",
            BrowserErrorInfo { category, message }.message
        )
    }

    /// Build the CDP HTTP endpoint URL.
    fn http_url(&self, path: &str) -> String {
        format!("http://{}:{}/{}", self.host, self.port, path)
    }

    async fn fetch_tabs_raw(&self) -> Result<Vec<RawCdpTab>> {
        let url = self.http_url("json/list");
        let client = reqwest::Client::builder()
            .timeout(self.action_timeout)
            .build()
            .context("Failed to build HTTP client for CDP tabs request")?;
        let resp = timeout(self.action_timeout, client.get(&url).send())
            .await
            .map_err(|_| Self::normalize_error(format!("Timeout fetching CDP tabs from {url}")))?
            .map_err(|err| Self::normalize_error(format!("Failed fetching CDP tabs: {err}")))?;
        if !resp.status().is_success() {
            return Err(Self::normalize_error(format!(
                "CDP endpoint returned {} while fetching tabs from {}",
                resp.status(),
                url
            )));
        }
        timeout(self.action_timeout, resp.json())
            .await
            .map_err(|_| Self::normalize_error(format!("Timeout decoding CDP tabs from {url}")))?
            .map_err(|err| {
                Self::normalize_error(format!("Failed to decode CDP tabs from {url}: {err}"))
            })
    }

    async fn resolve_active_page(&self) -> Result<RawCdpTab> {
        let tabs = self.fetch_tabs_raw().await?;
        let pages: Vec<_> = tabs.into_iter().filter(|t| t.type_ == "page").collect();
        if pages.is_empty() {
            return Err(Self::normalize_error(format!(
                "No page targets available on {}:{} (is {:?} running with --remote-debugging-port={})",
                self.host, self.port, self.browser_type, self.port
            )));
        }

        if let Some(target_id) = &self.target_id
            && let Some(tab) = pages.iter().find(|t| &t.id == target_id)
        {
            return Ok(tab.clone());
        }

        if let Some(ws_url) = &self.ws_url
            && let Some(tab) = pages.iter().find(|t| &t.ws_url == ws_url)
        {
            return Ok(tab.clone());
        }

        pages
            .into_iter()
            .find(|t| !t.ws_url.is_empty())
            .ok_or_else(|| {
                Self::normalize_error("No debuggable page target found (missing websocket URL)")
            })
    }

    async fn cdp_call_with_ws(&self, ws_url: &str, method: &str, params: Value) -> Result<Value> {
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..=self.max_retries {
            match self
                .cdp_call_with_ws_once(ws_url, method, params.clone())
                .await
            {
                Ok(value) => return Ok(value),
                Err(err) => {
                    let category = Self::classify_error_message(&err.to_string());
                    last_err = Some(err);
                    let can_retry =
                        category == BrowserErrorCategory::Transient && attempt < self.max_retries;
                    if can_retry {
                        let delay_ms = RETRY_BACKOFF_MS * (attempt as u64 + 1);
                        sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    break;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            Self::normalize_error(format!("Unknown CDP websocket error for method {method}"))
        }))
    }

    async fn cdp_call_with_ws_once(
        &self,
        ws_url: &str,
        method: &str,
        params: Value,
    ) -> Result<Value> {
        let (mut socket, _) = timeout(self.action_timeout, connect_async(ws_url))
            .await
            .map_err(|_| {
                Self::normalize_error(format!(
                    "Timeout connecting CDP websocket for {method}: {ws_url}"
                ))
            })?
            .map_err(|err| {
                Self::normalize_error(format!("Failed connecting CDP websocket: {ws_url}: {err}"))
            })?;

        let req_id = 1_u64;
        let request = json!({
            "id": req_id,
            "method": method,
            "params": params
        });

        timeout(
            self.action_timeout,
            socket.send(Message::Text(request.to_string().into())),
        )
        .await
        .map_err(|_| Self::normalize_error(format!("Timeout sending CDP request {method}")))?
        .map_err(|err| {
            Self::normalize_error(format!("Failed sending CDP request {method}: {err}"))
        })?;

        loop {
            let frame_opt = timeout(self.action_timeout, socket.next())
                .await
                .map_err(|_| {
                    Self::normalize_error(format!(
                        "Timeout waiting for CDP response frame for {method}"
                    ))
                })?;

            let Some(frame) = frame_opt else {
                return Err(Self::normalize_error(format!(
                    "CDP websocket closed before response for method {method}"
                )));
            };

            let frame = frame.map_err(|err| {
                Self::normalize_error(format!("Error receiving CDP response for {method}: {err}"))
            })?;

            match frame {
                Message::Text(txt) => {
                    if let Some(response) = Self::parse_cdp_response(&txt, req_id, method)? {
                        return Ok(response);
                    }
                }
                Message::Binary(bin) => {
                    let txt = String::from_utf8(bin.to_vec()).map_err(|err| {
                        Self::normalize_error(format!(
                            "CDP websocket returned non-utf8 binary payload: {err}"
                        ))
                    })?;
                    if let Some(response) = Self::parse_cdp_response(&txt, req_id, method)? {
                        return Ok(response);
                    }
                }
                Message::Ping(payload) => {
                    let _ = socket.send(Message::Pong(payload)).await;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
                Message::Close(_) => {
                    return Err(Self::normalize_error(format!(
                        "CDP websocket closed before response for method {method}"
                    )));
                }
            }
        }
    }

    fn parse_cdp_response(txt: &str, req_id: u64, method: &str) -> Result<Option<Value>> {
        let value: Value = serde_json::from_str(txt).map_err(|err| {
            Self::normalize_error(format!("Invalid JSON frame from CDP websocket: {err}"))
        })?;
        let Some(id) = value.get("id").and_then(Value::as_u64) else {
            return Ok(None);
        };
        if id != req_id {
            return Ok(None);
        }

        if let Some(err) = value.get("error") {
            let code = err.get("code").and_then(Value::as_i64).unwrap_or(-1);
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown CDP error");
            let detail = err
                .get("data")
                .map_or(String::new(), |d| format!("; data={}", d));
            return Err(Self::normalize_error(format!(
                "CDP {method} failed ({code}): {message}{detail}"
            )));
        }

        Ok(Some(
            value.get("result").cloned().unwrap_or_else(|| json!({})),
        ))
    }

    async fn cdp_call(&self, method: &str, params: Value) -> Result<Value> {
        let tab = self.resolve_active_page().await?;
        if tab.ws_url.is_empty() {
            return Err(Self::normalize_error(
                "Active tab has no websocket debugger URL",
            ));
        }
        self.cdp_call_with_ws(&tab.ws_url, method, params).await
    }

    async fn evaluate_json(&self, script: &str) -> Result<Value> {
        let result = self
            .cdp_call(
                "Runtime.evaluate",
                json!({
                    "expression": script,
                    "awaitPromise": true,
                    "returnByValue": true,
                }),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("JavaScript exception");
            return Err(Self::normalize_error(format!(
                "Runtime.evaluate exception: {text}"
            )));
        }

        let js_value = result
            .get("result")
            .and_then(|obj| obj.get("value"))
            .cloned()
            .unwrap_or(Value::Null);
        Ok(js_value)
    }

    fn stringify_json(value: Value) -> Result<String> {
        match value {
            Value::String(s) => Ok(s),
            other => serde_json::to_string(&other).context("Failed to stringify JavaScript value"),
        }
    }

    /// List all browser tabs via CDP /json endpoint.
    pub async fn list_tabs(&self) -> Result<Vec<CdpTab>> {
        let tabs = self.fetch_tabs_raw().await?;
        Ok(tabs.into_iter().filter_map(|t| t.to_tab()).collect())
    }

    /// Attach to a specific tab by ID.
    pub async fn attach_to_tab(&mut self, tab_id: &str) -> Result<()> {
        let tabs = self.fetch_tabs_raw().await?;
        let tab = tabs
            .into_iter()
            .find(|t| t.id == tab_id && t.type_ == "page")
            .ok_or_else(|| Self::normalize_error(format!("Tab not found: {tab_id}")))?;

        if tab.ws_url.is_empty() {
            return Err(Self::normalize_error(format!(
                "Tab {tab_id} cannot be debugged (missing websocket URL)"
            )));
        }

        self.ws_url = Some(tab.ws_url);
        self.target_id = Some(tab.id);
        Ok(())
    }

    /// Create a new browser tab (CDP NewTarget).
    pub async fn new_tab(&mut self, url: &str) -> Result<String> {
        let endpoint = self.http_url(&format!("json/new?{url}"));
        let client = reqwest::Client::builder()
            .timeout(self.action_timeout)
            .build()
            .context("Failed to build HTTP client for CDP new-tab request")?;

        let response = match timeout(self.action_timeout, client.put(&endpoint).send()).await {
            Ok(Ok(resp)) if resp.status().is_success() => resp,
            _ => client.get(&endpoint).send().await.map_err(|err| {
                Self::normalize_error(format!("Failed creating new tab via {endpoint}: {err}"))
            })?,
        };

        if !response.status().is_success() {
            return Err(Self::normalize_error(format!(
                "Failed creating new tab: status={} endpoint={}",
                response.status(),
                endpoint
            )));
        }

        let tab: RawCdpTab = timeout(self.action_timeout, response.json())
            .await
            .map_err(|_| Self::normalize_error("Timeout decoding CDP new-tab response"))?
            .map_err(|err| {
                Self::normalize_error(format!("Failed decoding CDP new-tab response: {err}"))
            })?;
        if tab.id.is_empty() || tab.ws_url.is_empty() {
            return Err(Self::normalize_error(
                "CDP new-tab response missing id or websocket URL",
            ));
        }

        self.target_id = Some(tab.id.clone());
        self.ws_url = Some(tab.ws_url);
        Ok(tab.id)
    }

    /// Navigate to a URL in the active tab.
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        if self.resolve_active_page().await.is_err() {
            self.new_tab(url).await?;
            return Ok(());
        }

        let _ = self.cdp_call("Page.enable", json!({})).await;
        self.cdp_call("Page.navigate", json!({ "url": url }))
            .await?;
        Ok(())
    }

    /// Get interactable elements on the page.
    pub async fn get_interactables(&self) -> Result<Vec<CdpInteractable>> {
        let script = r#"
(() => {
  const cssEscape = (window.CSS && CSS.escape)
    ? CSS.escape
    : (v) => String(v).replace(/["\\]/g, "\\$&");

  const selectorFor = (el) => {
    if (!el || !(el instanceof Element)) return "";
    if (el.id) return `#${cssEscape(el.id)}`;
    if (el.getAttribute("name")) {
      return `${el.tagName.toLowerCase()}[name="${cssEscape(el.getAttribute("name"))}"]`;
    }

    const chain = [];
    let cur = el;
    while (cur && cur.nodeType === Node.ELEMENT_NODE && chain.length < 8) {
      let part = cur.tagName.toLowerCase();
      const classes = Array.from(cur.classList || []).filter(Boolean).slice(0, 2);
      if (classes.length > 0) {
        part += classes.map(c => "." + cssEscape(c)).join("");
      } else if (cur.parentElement) {
        const sibs = Array.from(cur.parentElement.children).filter(n => n.tagName === cur.tagName);
        if (sibs.length > 1) {
          part += `:nth-of-type(${sibs.indexOf(cur) + 1})`;
        }
      }
      chain.unshift(part);
      if (cur.id) break;
      cur = cur.parentElement;
    }
    return chain.join(" > ");
  };

  const isVisible = (el, rect) => {
    const style = window.getComputedStyle(el);
    return rect.width > 0 &&
      rect.height > 0 &&
      style.visibility !== "hidden" &&
      style.display !== "none" &&
      style.opacity !== "0";
  };

  const nodes = Array.from(document.querySelectorAll(
    "a, button, input, select, textarea, [role='button'], [onclick], [contenteditable='true']"
  ));

  return nodes.map((el) => {
    const rect = el.getBoundingClientRect();
    const txt = (el.innerText || el.textContent || "").trim();
    return {
      selector: selectorFor(el),
      tag: el.tagName.toLowerCase(),
      text: txt || null,
      rect: {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height
      },
      input_type: el.getAttribute("type"),
      visible: isVisible(el, rect)
    };
  });
})()
"#;

        let value = self.evaluate_json(script).await?;
        serde_json::from_value(value).context("Failed parsing interactables response")
    }

    /// Fill form fields on the page.
    pub async fn fill_form(&self, fields: &[CdpFormField]) -> Result<()> {
        if fields.is_empty() {
            return Ok(());
        }

        let fields_json =
            serde_json::to_string(fields).context("Failed serializing fields for fill_form")?;
        let script = format!(
            r#"
(() => {{
  const fields = {fields_json};
  const cssEscape = (window.CSS && CSS.escape)
    ? CSS.escape
    : (v) => String(v).replace(/["\\]/g, "\\$&");

  const resolveElement = (field) => {{
    if (field.selector) {{
      const el = document.querySelector(field.selector);
      if (el) return {{ el, selector: field.selector }};
    }}
    if (field.id) {{
      const byId = document.getElementById(field.id);
      if (byId) return {{ el: byId, selector: `#${{cssEscape(field.id)}}` }};
    }}
    if (field.name) {{
      const byName = document.querySelector(`[name="${{cssEscape(field.name)}}"]`);
      if (byName) return {{ el: byName, selector: `[name="${{cssEscape(field.name)}}"]` }};
    }}
    if (field.placeholder) {{
      const byPlaceholder = document.querySelector(`[placeholder="${{cssEscape(field.placeholder)}}"]`);
      if (byPlaceholder) return {{ el: byPlaceholder, selector: `[placeholder="${{cssEscape(field.placeholder)}}"]` }};
    }}
    return {{ el: null, selector: field.selector || field.id || field.name || field.placeholder || "<unknown>" }};
  }};

  const output = {{
    filled: [],
    missing: [],
    errors: []
  }};

  for (const field of fields) {{
    const {{ el, selector }} = resolveElement(field);
    if (!el) {{
      output.missing.push(selector);
      continue;
    }}

    try {{
      const desiredValue = field.value ?? "";
      const inputType = String(field.input_type || el.type || "").toLowerCase();
      const tag = String(el.tagName || "").toLowerCase();
      const boolValue = ["1", "true", "yes", "on"].includes(String(desiredValue).toLowerCase());

      if (inputType === "checkbox" || inputType === "radio") {{
        el.checked = boolValue;
      }} else if (tag === "select") {{
        el.value = String(desiredValue);
      }} else {{
        el.focus();
        el.value = String(desiredValue);
      }}

      el.dispatchEvent(new Event("input", {{ bubbles: true }}));
      el.dispatchEvent(new Event("change", {{ bubbles: true }}));
      output.filled.push(selector);
    }} catch (err) {{
      output.errors.push(`${{selector}}: ${{err?.message || String(err)}}`);
    }}
  }}

  return output;
}})()
"#
        );

        let value = self.evaluate_json(&script).await?;
        let missing = value
            .get("missing")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let errors = value
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if !errors.is_empty() {
            bail!("Failed filling fields: {}", Value::Array(errors));
        }
        if !missing.is_empty() {
            bail!("Some fields were not found: {}", Value::Array(missing));
        }
        Ok(())
    }

    /// Click an element by selector.
    pub async fn click(&self, selector: &str) -> Result<()> {
        let selector_json =
            serde_json::to_string(selector).context("Failed serializing click selector")?;
        let script = format!(
            r#"
(() => {{
  const raw = {selector_json};
  const isVisible = (el) => {{
    if (!el) return false;
    const rect = el.getBoundingClientRect();
    const style = window.getComputedStyle(el);
    return rect.width > 0 &&
      rect.height > 0 &&
      style.visibility !== "hidden" &&
      style.display !== "none" &&
      style.opacity !== "0";
  }};

  const clickable = () => Array.from(document.querySelectorAll(
    "a, button, input[type='button'], input[type='submit'], [role='button'], [onclick]"
  )).filter(isVisible);

  const cssEscape = (window.CSS && CSS.escape)
    ? CSS.escape
    : (v) => String(v).replace(/["\\]/g, "\\$&");
  const selectorFor = (el) => {{
    if (!el || !(el instanceof Element)) return "";
    if (el.id) return `#${{cssEscape(el.id)}}`;
    if (el.getAttribute("name")) {{
      return `${{el.tagName.toLowerCase()}}[name="${{cssEscape(el.getAttribute("name"))}}"]`;
    }}
    return el.tagName.toLowerCase();
  }};

  let target = null;
  if (raw.startsWith("text=")) {{
    const needle = raw.slice(5).trim().toLowerCase();
    const matches = clickable().filter((el) => (el.innerText || el.textContent || "")
      .trim()
      .toLowerCase()
      .includes(needle));
    if (matches.length > 1) {{
      const candidates = matches.slice(0, 8).map((el) => ({{
        selector: selectorFor(el),
        tag: (el.tagName || "").toLowerCase(),
        text: (el.innerText || el.textContent || "").trim().slice(0, 120)
      }}));
      throw new Error(`Ambiguous click target for selector: ${{raw}}; candidates=${{JSON.stringify(candidates)}}`);
    }}
    target = matches[0] || null;
  }} else if (raw.startsWith("index=")) {{
    const idx = Number(raw.slice(6));
    if (!Number.isNaN(idx)) {{
      target = clickable()[idx] || null;
    }}
  }} else {{
    target = document.querySelector(raw);
  }}

  if (!target) {{
    throw new Error(`No clickable element found for selector: ${{raw}}`);
  }}
  if (!isVisible(target)) {{
    throw new Error(`Element is not visible for selector: ${{raw}}`);
  }}

  target.scrollIntoView({{ block: "center", inline: "center" }});
  target.click();
  return "clicked";
}})()
"#
        );

        let _ = self.evaluate_json(&script).await?;
        Ok(())
    }

    /// Evaluate a JavaScript expression.
    pub async fn evaluate(&self, script: &str) -> Result<String> {
        let value = self.evaluate_json(script).await?;
        Self::stringify_json(value)
    }

    /// Take a screenshot of the current tab.
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        let _ = self.cdp_call("Page.enable", json!({})).await;
        let result = self
            .cdp_call(
                "Page.captureScreenshot",
                json!({
                    "format": "png",
                    "fromSurface": true
                }),
            )
            .await?;

        let encoded = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Page.captureScreenshot returned no data field"))?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .context("Failed to decode screenshot base64 payload")
    }

    /// Get page HTML content.
    pub async fn get_content(&self) -> Result<String> {
        self.evaluate("document.documentElement ? document.documentElement.outerHTML : ''")
            .await
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
    if reqwest::get(chrome.http_url("json/list")).await.is_ok() {
        browsers.push(chrome);
    }

    // Try Edge default port (9223 is sometimes used)
    let mut edge = CdpBrowser::new(BrowserType::Edge);
    edge.port = 9223;
    if reqwest::get(edge.http_url("json/list")).await.is_ok() {
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

    #[test]
    fn raw_tab_maps_to_page_tab() {
        let raw = RawCdpTab {
            id: "target-1".to_string(),
            title: "Example".to_string(),
            url: "https://example.com".to_string(),
            type_: "page".to_string(),
            ws_url: "ws://127.0.0.1:9222/devtools/page/target-1".to_string(),
        };
        let tab = raw.to_tab().expect("page should convert");
        assert_eq!(tab.id, "target-1");
        assert_eq!(
            tab.ws_url.as_deref(),
            Some("ws://127.0.0.1:9222/devtools/page/target-1")
        );
    }

    #[test]
    fn parse_cdp_response_ignores_events() {
        let event = r#"{"method":"Runtime.consoleAPICalled","params":{"type":"log"}}"#;
        let parsed = CdpBrowser::parse_cdp_response(event, 1, "Runtime.evaluate")
            .expect("event should parse");
        assert!(parsed.is_none());
    }

    #[test]
    fn stringify_json_formats_non_strings() {
        let out = CdpBrowser::stringify_json(json!({"ok": true})).expect("stringify");
        assert_eq!(out, r#"{"ok":true}"#);
    }

    #[test]
    fn browser_defaults_include_resilience_controls() {
        let browser = CdpBrowser::new(BrowserType::Chrome);
        assert_eq!(browser.action_timeout, DEFAULT_ACTION_TIMEOUT);
        assert_eq!(browser.max_retries, DEFAULT_MAX_RETRIES);
    }

    #[test]
    fn classify_error_message_maps_expected_categories() {
        assert_eq!(
            CdpBrowser::classify_error_message("request timeout while connecting"),
            BrowserErrorCategory::Transient
        );
        assert_eq!(
            CdpBrowser::classify_error_message("invalid selector syntax"),
            BrowserErrorCategory::InvalidInput
        );
        assert_eq!(
            CdpBrowser::classify_error_message("tab not found"),
            BrowserErrorCategory::TargetMissing
        );
        assert_eq!(
            CdpBrowser::classify_error_message("unexpected protocol mismatch"),
            BrowserErrorCategory::Fatal
        );
    }

    #[test]
    fn normalize_error_prefixes_category() {
        let err = CdpBrowser::normalize_error("tab not found");
        let msg = err.to_string();
        assert!(msg.contains("[browser/TargetMissing]"));
    }

    #[tokio::test]
    async fn list_tabs_returns_error_on_no_browser() {
        let browser = CdpBrowser::new(BrowserType::Chrome).with_port(9999);
        let tabs = browser.list_tabs().await;
        // Should error because nothing is on port 9999
        assert!(tabs.is_err());
    }
}
