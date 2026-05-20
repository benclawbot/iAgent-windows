//! `app` tool - unified Office/browser automation via COM and CDP.
//!
//! Provides a single tool entry point wrapping all app-integrations:
//! - Word (COM): document creation, text manipulation, formatting
//! - Excel (COM): spreadsheet operations, cell read/write, formulas
//! - PowerPoint (COM): slide analysis, design suggestions, presentation control
//! - Browser (CDP): Chrome/Edge via DevTools Protocol over HTTP/WebSocket
//! - Form Fill (CDP): high-level form filling via CDP
//!
//! ## Actions
//!
//! ### Word
//! - `word_connect` - Connect to Word via COM
//! - `word_get_selection` - Get current selection text
//! - `word_set_selection` - Replace current selection
//! - `word_get_document` - Get active document content
//! - `word_insert_text` - Insert text at cursor
//! - `word_bold` - Bold the selection
//! - `word_save` / `word_close` - Save or close document
//!
//! ### Excel
//! - `excel_connect` - Connect to Excel via COM
//! - `excel_get_cell` - Read a cell value
//! - `excel_set_cell` - Write to a cell
//! - `excel_set_formula` - Set a cell formula
//! - `excel_used_range` - Get used range dimensions
//! - `excel_evaluate` - Evaluate a formula
//! - `excel_save` / `excel_close` - Save or close workbook
//!
//! ### PowerPoint
//! - `ppt_connect` - Connect to PowerPoint via COM
//! - `ppt_get_slide` - Get active slide content
//! - `ppt_suggest` - Get design improvement suggestions
//! - `ppt_save` / `ppt_close` - Save or close presentation
//!
//! ### Browser (CDP)
//! - `browser_status` - Check CDP browser status
//! - `browser_list_tabs` - List open tabs
//! - `browser_new_tab` - Open a new tab
//! - `browser_navigate` - Navigate to URL
//! - `browser_get_content` - Get page HTML
//! - `browser_interactables` - Get clickable elements
//! - `browser_click` - Click an element
//! - `browser_type` - Type text
//! - `browser_fill` - Fill form fields
//! - `browser_screenshot` - Take screenshot
//! - `browser_evaluate` - Execute JavaScript
//!
//! ### Form Fill
//! - `form_fill` - Fill a multi-field form with submit option

use crate::tool::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use app_integrations::{browser, excel, form_fill, powerpoint, word};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct AppTool;

impl AppTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum AppInput {
    // Word actions
    word_connect,
    word_get_selection,
    word_set_selection { text: String },
    word_get_document,
    word_insert_text { document_id: Option<String>, text: String },
    word_bold,
    word_save { document_id: Option<String> },
    word_close { document_id: Option<String>, save_changes: Option<bool> },

    // Excel actions
    excel_connect,
    excel_get_cell { sheet: String, row: u32, col: u32 },
    excel_set_cell { sheet: String, row: u32, col: u32, value: String },
    excel_set_formula { sheet: String, row: u32, col: u32, formula: String },
    excel_used_range { sheet: String },
    excel_evaluate { formula: String },
    excel_save,
    excel_close { save_changes: Option<bool> },

    // PowerPoint actions
    ppt_connect,
    ppt_get_slide,
    ppt_suggest { content: String },
    ppt_save,
    ppt_close,

    // Browser (CDP) actions
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

    // Form fill action
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
        "Office automation (Word/Excel/PowerPoint via COM) and browser automation (Chrome/Edge \
         via CDP). Actions: word_*, excel_*, ppt_*, browser_*, form_fill. \
         Use 'form_fill' for multi-field form filling with smart selector matching."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "intent": { "type": "string" },
                "action": {
                    "type": "string",
                    "enum": [
                        "word_connect", "word_get_selection", "word_set_selection",
                        "word_get_document", "word_insert_text", "word_bold",
                        "word_save", "word_close",
                        "excel_connect", "excel_get_cell", "excel_set_cell",
                        "excel_set_formula", "excel_used_range", "excel_evaluate",
                        "excel_save", "excel_close",
                        "ppt_connect", "ppt_get_slide", "ppt_suggest",
                        "ppt_save", "ppt_close",
                        "browser_list_tabs", "browser_new_tab", "browser_navigate",
                        "browser_get_content", "browser_screenshot",
                        "browser_interactables", "browser_click", "browser_type",
                        "browser_evaluate", "browser_fill",
                        "form_fill",
                    ],
                    "description": "The app automation action to perform."
                },
                // Word params
                "text": { "type": "string" },
                "save_changes": { "type": "boolean" },
                // Excel params
                "sheet": { "type": "string" },
                "row": { "type": "integer" },
                "col": { "type": "integer" },
                "value": { "type": "string" },
                "formula": { "type": "string" },
                // PowerPoint params
                "content": { "type": "string" },
                // Browser params
                "port": { "type": "integer" },
                "url": { "type": "string" },
                "selector": { "type": "string" },
                "script": { "type": "string" },
                "fields": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "selector": { "type": "string" },
                            "name": { "type": "string" },
                            "id": { "type": "string" },
                            "label": { "type": "string" },
                            "placeholder": { "type": "string" },
                            "value": { "type": "string" },
                            "checked": { "type": "boolean" },
                        }
                    }
                },
                // Form fill params
                "request": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "fields": { "$ref": "#/properties/fields" },
                        "submit": { "type": "boolean" },
                        "submit_selector": { "type": "string" },
                        "wait_after_submit_ms": { "type": "integer" },
                    }
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let input: AppInput = serde_json::from_value(input)?;

        match input {
            // ===== Word =====
            AppInput::word_connect => {
                let word = word::WordIntegration::connect()?;
                Ok(ToolOutput::new(format!(
                    "Word connection: connected={}",
                    word.is_connected()
                )))
            }
            AppInput::word_get_selection => {
                let w = word::WordIntegration::connect()?;
                let sel = w.get_selection()?;
                Ok(ToolOutput::new(serde_json::to_string(&sel)?))
            }
            AppInput::word_set_selection { text } => {
                let w = word::WordIntegration::connect()?;
                w.set_selection_text(&text)?;
                Ok(ToolOutput::new("Selection updated"))
            }
            AppInput::word_get_document => {
                let w = word::WordIntegration::connect()?;
                let content = w.get_active_document_content()?;
                Ok(ToolOutput::new(content))
            }
            AppInput::word_insert_text { document_id, text } => {
                let w = word::WordIntegration::connect()?;
                let doc_id = document_id.as_deref().unwrap_or("");
                w.insert_text(doc_id, &text)?;
                Ok(ToolOutput::new("Text inserted"))
            }
            AppInput::word_bold => {
                let w = word::WordIntegration::connect()?;
                w.bold_selection()?;
                Ok(ToolOutput::new("Bold formatting applied"))
            }
            AppInput::word_save { document_id } => {
                let w = word::WordIntegration::connect()?;
                let doc_id = document_id.as_deref().unwrap_or("");
                w.save(doc_id)?;
                Ok(ToolOutput::new("Document saved"))
            }
            AppInput::word_close { document_id, save_changes } => {
                let w = word::WordIntegration::connect()?;
                let doc_id = document_id.as_deref().unwrap_or("");
                w.close(doc_id)?;
                Ok(ToolOutput::new("Document closed"))
            }

            // ===== Excel =====
            AppInput::excel_connect => {
                let e = excel::ExcelIntegration::connect()?;
                Ok(ToolOutput::new(format!(
                    "Excel connection: connected={}",
                    e.is_connected()
                )))
            }
            AppInput::excel_get_cell { sheet, row, col } => {
                let e = excel::ExcelIntegration::connect()?;
                let cell = e.get_cell(&sheet, row, col)?;
                Ok(ToolOutput::new(serde_json::to_string(&cell)?))
            }
            AppInput::excel_set_cell { sheet, row, col, value } => {
                let e = excel::ExcelIntegration::connect()?;
                e.set_cell(&sheet, row, col, &value)?;
                Ok(ToolOutput::new("Cell updated"))
            }
            AppInput::excel_set_formula { sheet, row, col, formula } => {
                let e = excel::ExcelIntegration::connect()?;
                e.set_formula(&sheet, row, col, &formula)?;
                Ok(ToolOutput::new("Formula set"))
            }
            AppInput::excel_used_range { sheet } => {
                let e = excel::ExcelIntegration::connect()?;
                let (rows, cols) = e.get_used_range(&sheet)?;
                Ok(ToolOutput::new(format!("Rows: {}, Cols: {}", rows, cols)))
            }
            AppInput::excel_evaluate { formula } => {
                let e = excel::ExcelIntegration::connect()?;
                let result = e.evaluate(&formula)?;
                Ok(ToolOutput::new(result))
            }
            AppInput::excel_save => {
                let e = excel::ExcelIntegration::connect()?;
                e.save()?;
                Ok(ToolOutput::new("Workbook saved"))
            }
            AppInput::excel_close { save_changes } => {
                let e = excel::ExcelIntegration::connect()?;
                e.close(save_changes.unwrap_or(true))?;
                Ok(ToolOutput::new("Workbook closed"))
            }

            // ===== PowerPoint =====
            AppInput::ppt_connect => {
                let p = powerpoint::PowerPointIntegration::connect()?;
                Ok(ToolOutput::new(format!(
                    "PowerPoint connection: connected={}",
                    p.is_connected()
                )))
            }
            AppInput::ppt_get_slide => {
                let p = powerpoint::PowerPointIntegration::connect()?;
                let slide = p.get_active_slide_content()?;
                Ok(ToolOutput::new(serde_json::to_string(&slide)?))
            }
            AppInput::ppt_suggest { content } => {
                let p = powerpoint::PowerPointIntegration::connect()?;
                let suggestions = p.suggest_design_improvements(&content);
                Ok(ToolOutput::new(serde_json::to_string(&suggestions)?))
            }
            AppInput::ppt_save => {
                let p = powerpoint::PowerPointIntegration::connect()?;
                p.save()?;
                Ok(ToolOutput::new("Presentation saved"))
            }
            AppInput::ppt_close => {
                let p = powerpoint::PowerPointIntegration::connect()?;
                p.close()?;
                Ok(ToolOutput::new("Presentation closed"))
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
                Ok(ToolOutput::new(format!("Screenshot: {} bytes", data.len())).with_metadata(
                    serde_json::json!({ "screenshot_b64": b64 }),
                ))
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
                // type uses fill_form with a single field
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