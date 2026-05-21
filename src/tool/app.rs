//! `app` tool — Office document automation via OfficeCLI + browser automation via CDP.
//!
//! Office: Word (.docx), Excel (.xlsx), PowerPoint (.pptx) via OfficeCLI subprocess.
//! Browser: Chrome/Edge via Chrome DevTools Protocol (CDP/HTTP/WebSocket).
//!
//! ## OfficeCLI Installation
//!
//! ```bash
//! # macOS / Linux
//! curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
//!
//! # Windows (PowerShell)
//! irm https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.ps1 | iex
//! ```
//!
//! Verify: `officecli --version`
//!
//! ## Browser Setup
//!
//! Chrome/Edge must be launched with `--remote-debugging-port=9222`.

use crate::tool::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use app_integrations::{browser, form_fill, officecli};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct AppTool;

impl AppTool {
    pub fn new() -> Self {
        Self
    }

    /// Returns true if OfficeCLI is available on this system.
    pub fn officecli_ready() -> bool {
        app_integrations::officecli::is_installed()
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum AppInput {
    // ===== OfficeCLI / Office =====

    /// Check if OfficeCLI is installed.
    oc_check,

    /// Get OfficeCLI version.
    oc_version,

    /// Create a blank Office document.
    oc_create { path: String },

    /// View document (modes: outline, stats, issues, text, annotated, html).
    oc_view { path: String, mode: String },

    /// Get document statistics as JSON.
    oc_stats { path: String },

    /// Validate a document against OpenXML schema.
    oc_validate { path: String },

    /// Extract plain text from a document.
    oc_text { path: String, start: Option<u32>, end: Option<u32> },

    /// Get a node at a path.
    oc_get { path: String, doc_path: String, depth: Option<u32>, json: Option<bool> },

    /// Query a document with a CSS-like selector.
    oc_query { path: String, selector: String, json: Option<bool> },

    /// Set properties on a document element.
    oc_set { path: String, doc_path: String, props: Vec<(String, String)> },

    /// Add an element to a document.
    oc_add { path: String, parent: String, element_type: String, props: Vec<(String, String)> },

    /// Format matched text (e.g. make text bold/red).
    oc_format { path: String, doc_path: String, find: String, props: Vec<(String, String)>, regex: Option<bool> },

    /// Replace matched text throughout document.
    oc_replace { path: String, doc_path: String, find: String, replacement: String, regex: Option<bool> },

    /// Remove an element.
    oc_remove { path: String, doc_path: String },

    /// Open document in resident mode.
    oc_open { path: String },

    /// Close document (flush changes).
    oc_close { path: String },

    /// Run batch operations from JSON.
    oc_batch { path: String, commands: String, json: Option<bool> },

    /// Export document as HTML.
    oc_export_html { path: String, browser: Option<bool> },

    // ===== Browser (CDP) =====

    browser_list_tabs { port: Option<u16> },
    browser_new_tab { url: String, port: Option<u16> },
    browser_navigate { url: String, port: Option<u16> },
    browser_get_content { port: Option<u16> },
    browser_screenshot { port: Option<u16> },
    browser_interactables { port: Option<u16> },
    browser_click { selector: String, port: Option<u16> },
    browser_type { text: String, port: Option<u16> },
    browser_evaluate { script: String, port: Option<u16> },
    browser_fill { fields: Vec<form_fill::FormField>, url: Option<String>, port: Option<u16> },

    // ===== Form Fill =====

    form_fill { request: form_fill::FormFillRequest },
}

fn make_browser(port: u16) -> browser::CdpBrowser {
    browser::CdpBrowser::new(browser::BrowserType::Chrome).with_port(port)
}

#[async_trait]
impl Tool for AppTool {
    fn name(&self) -> &str {
        "app"
    }

    fn description(&self) -> &str {
        "Office document automation (Word/Excel/PowerPoint via OfficeCLI) and browser \
         automation (Chrome/Edge via CDP). Office: oc_* actions. Browser: browser_* / form_fill."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "description": "The action to perform. Office actions start with 'oc_'. Browser actions start with 'browser_'. Form fill: 'form_fill'."
                },
                // oc_ params
                "path": { "type": "string" },
                "mode": { "type": "string" },
                "doc_path": { "type": "string" },
                "depth": { "type": "integer" },
                "json": { "type": "boolean" },
                "props": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "k": { "type": "string" },
                            "v": { "type": "string" }
                        },
                        "required": ["k", "v"]
                    }
                },
                "element_type": { "type": "string" },
                "parent": { "type": "string" },
                "find": { "type": "string" },
                "replacement": { "type": "string" },
                "regex": { "type": "boolean" },
                "start": { "type": "integer" },
                "end": { "type": "integer" },
                "commands": { "type": "string" },
                // browser params
                "port": { "type": "integer" },
                "url": { "type": "string" },
                "selector": { "type": "string" },
                "script": { "type": "string" },
                "text": { "type": "string" },
                "fields": { "type": "array" },
                // form fill params
                "request": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "fields": { "$ref": "#/properties/fields" },
                        "submit": { "type": "boolean" },
                    }
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: AppInput = serde_json::from_value(input)?;

        match input {
            // ===== OfficeCLI / Office =====

            AppInput::oc_check => {
                Ok(ToolOutput::new(format!("OfficeCLI installed: {}", officecli::is_installed())))
            }

            AppInput::oc_version => {
                let v = officecli::version()?;
                Ok(ToolOutput::new(v))
            }

            AppInput::oc_create { path } => {
                let out = officecli::create(&path)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_view { path, mode } => {
                let out = officecli::view(&path, &mode)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_stats { path } => {
                match officecli::stats(&path) {
                    Ok(s) => Ok(ToolOutput::new(serde_json::to_string_pretty(&s)?)),
                    Err(e) => Ok(ToolOutput::new(format!("Stats unavailable: {}", e))),
                }
            }

            AppInput::oc_validate { path } => {
                match officecli::validate(&path) {
                    Ok(v) => Ok(ToolOutput::new(serde_json::to_string_pretty(&v)?)),
                    Err(e) => Ok(ToolOutput::new(format!("Validation failed: {}", e))),
                }
            }

            AppInput::oc_text { path, start, end } => {
                let out = officecli::text(&path, start, end)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_get { path, doc_path, depth, json } => {
                let out = officecli::get(&path, &doc_path, depth, json.unwrap_or(false))?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_query { path, selector, json } => {
                let out = officecli::query(&path, &selector, json.unwrap_or(false))?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_set { path, doc_path, props } => {
                let props_ref: Vec<(&str, &str)> = props.iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                let out = officecli::set(&path, &doc_path, &props_ref)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_add { path, parent, element_type, props } => {
                let props_ref: Vec<(&str, &str)> = props.iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                let out = officecli::add(&path, &parent, &element_type, &props_ref)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_format { path, doc_path, find, props, regex } => {
                let props_ref: Vec<(&str, &str)> = props.iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                let out = officecli::format_text(&path, &doc_path, &find, &props_ref, regex.unwrap_or(false))?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_replace { path, doc_path, find, replacement, regex } => {
                let out = officecli::replace(&path, &doc_path, &find, &replacement, regex.unwrap_or(false))?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_remove { path, doc_path } => {
                let out = officecli::remove(&path, &doc_path)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_open { path } => {
                let out = officecli::open(&path)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_close { path } => {
                let out = officecli::close(&path)?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_batch { path, commands, json } => {
                let out = officecli::batch(&path, &commands, json.unwrap_or(false))?;
                Ok(ToolOutput::new(out))
            }

            AppInput::oc_export_html { path, browser } => {
                let out = officecli::export_html(&path, browser.unwrap_or(false))?;
                Ok(ToolOutput::new(out))
            }

            // ===== Browser (CDP) =====

            AppInput::browser_list_tabs { port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                let tabs = b.list_tabs().await?;
                Ok(ToolOutput::new(serde_json::to_string(&tabs)?))
            }

            AppInput::browser_new_tab { url, port } => {
                let port = port.unwrap_or(9222);
                let mut b = make_browser(port);
                let tab_id = b.new_tab(&url).await?;
                Ok(ToolOutput::new(format!("New tab: {}", tab_id)))
            }

            AppInput::browser_navigate { url, port } => {
                let port = port.unwrap_or(9222);
                let mut b = make_browser(port);
                b.navigate(&url).await?;
                Ok(ToolOutput::new(format!("Navigated to {}", url)))
            }

            AppInput::browser_get_content { port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                let content = b.get_content().await?;
                Ok(ToolOutput::new(content))
            }

            AppInput::browser_screenshot { port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                let data = b.screenshot().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                Ok(ToolOutput::new(format!("Screenshot: {} bytes", data.len()))
                    .with_metadata(serde_json::json!({ "screenshot_b64": b64 })))
            }

            AppInput::browser_interactables { port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                let els = b.get_interactables().await?;
                Ok(ToolOutput::new(serde_json::to_string(&els)?))
            }

            AppInput::browser_click { selector, port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                b.click(&selector).await?;
                Ok(ToolOutput::new(format!("Clicked: {}", selector)))
            }

            AppInput::browser_type { text, port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                use browser::CdpFormField;
                b.fill_form(&[CdpFormField {
                    selector: String::new(),
                    value: Some(text),
                    input_type: "text".to_string(),
                    name: None,
                    id: None,
                    placeholder: None,
                    required: false,
                    visible: true,
                }]).await?;
                Ok(ToolOutput::new("Typed"))
            }

            AppInput::browser_evaluate { script, port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                let result = b.evaluate(&script).await?;
                Ok(ToolOutput::new(result))
            }

            AppInput::browser_fill { fields, url, port } => {
                let port = port.unwrap_or(9222);
                let b = make_browser(port);
                use browser::CdpFormField;
                let cdp_fields: Vec<_> = fields
                    .into_iter()
                    .map(|f| CdpFormField {
                        selector: f.selector.unwrap_or_default(),
                        value: Some(f.value),
                        input_type: "text".to_string(),
                        name: f.name,
                        id: f.id,
                        placeholder: f.placeholder,
                        required: f.checked.is_some(),
                        visible: true,
                    })
                    .collect();
                b.fill_form(&cdp_fields).await?;
                Ok(ToolOutput::new(format!("Filled {} field(s)", cdp_fields.len())))
            }

            // ===== Form Fill =====

            AppInput::form_fill { request } => {
                let mut b = make_browser(9222);
                let result = form_fill::fill_form(&mut b, &request).await?;
                Ok(ToolOutput::new(serde_json::to_string(&result)?))
            }
        }
    }
}