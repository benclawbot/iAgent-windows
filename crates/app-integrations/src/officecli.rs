//! Office document integration via OfficeCLI subprocess.
//!
//! Wraps the OfficeCLI binary (https://github.com/iOfficeAI/OfficeCLI) for
//! cross-platform Word (.docx), Excel (.xlsx), and PowerPoint (.pptx) operations.
//! Single binary, no Office installation required.
//!
//! ## Installation
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

use std::io::Write as IoWrite;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// OfficeCLI executable name.
const CLI: &str = "officecli";

/// OfficeCLI document types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    Docx,
    Xlsx,
    Pptx,
}

impl DocType {
    pub fn from_extension(path: &str) -> Option<Self> {
        let p = Path::new(path);
        match p.extension()?.to_str()?.to_lowercase().as_str() {
            "docx" | "docm" | "dotx" => Some(DocType::Docx),
            "xlsx" | "xlsm" | "xltx" | "csv" => Some(DocType::Xlsx),
            "pptx" | "pptm" | "potx" => Some(DocType::Pptx),
            _ => None,
        }
    }

    pub fn cli_name(&self) -> &'static str {
        match self {
            DocType::Docx => "docx",
            DocType::Xlsx => "xlsx",
            DocType::Pptx => "pptx",
        }
    }
}

/// OfficeCLI output with success/failure info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Check if OfficeCLI is installed.
pub fn is_installed() -> bool {
    Command::new(CLI)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get OfficeCLI version string.
pub fn version() -> Result<String> {
    let output = Command::new(CLI)
        .arg("--version")
        .output()
        .context("Failed to run officecli --version")?;

    if !output.status.success() {
        bail!(
            "officecli --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run an officecli command. Returns stdout on success, or error with captured output.
fn run(args: &[&str]) -> Result<String> {
    let output = Command::new(CLI)
        .args(args)
        .output()
        .context(format!("officecli {} failed", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        bail!(
            "officecli {} failed (exit {}):\n{}\n{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stderr,
            stdout
        );
    }

    Ok(stdout)
}

/// Run officecli and capture full output (including failures).
#[allow(dead_code)]
fn run_capture(args: &[&str]) -> Result<CliOutput> {
    let output = Command::new(CLI)
        .args(args)
        .output()
        .context(format!("officecli {} failed to start", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(CliOutput {
        success: output.status.success(),
        stdout,
        stderr,
        exit_code,
    })
}

/// Run officecli with stdin input.
fn run_with_stdin(args: &[&str], input: &str) -> Result<String> {
    let mut child = Command::new(CLI)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(format!("officecli {} failed to start", args.join(" ")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .context("Failed to write to officecli stdin")?;
    }

    let output = child.wait_with_output().context("officecli wait failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        bail!(
            "officecli {} failed (exit {}):\n{}\n{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stderr,
            stdout
        );
    }

    Ok(stdout)
}

// =============================================================================
// Core operations
// =============================================================================

/// Create a blank Office document.
pub fn create(path: &str) -> Result<String> {
    run(&["create", path])
}

/// View document in a given mode.
pub fn view(path: &str, mode: &str) -> Result<String> {
    run(&["view", path, mode])
}

/// Get document statistics as JSON.
pub fn stats(path: &str) -> Result<CliStats> {
    let out = run(&["view", path, "stats", "--json"])?;
    serde_json::from_str(&out).context("Failed to parse stats JSON")
}

/// Validate a document against OpenXML schema.
pub fn validate(path: &str) -> Result<CliValidation> {
    let out = run(&["validate", path, "--json"])?;
    serde_json::from_str(&out).context("Failed to parse validation JSON")
}

/// Extract plain text from a document.
pub fn text(path: &str, start: Option<u32>, end: Option<u32>) -> Result<String> {
    let mut args: Vec<String> = vec!["view".into(), path.into(), "text".into()];
    if let Some(s) = start {
        args.push("--start".into());
        args.push(s.to_string());
    }
    if let Some(e) = end {
        args.push("--end".into());
        args.push(e.to_string());
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Get a node at a path, optionally with depth and JSON output.
pub fn get(path: &str, doc_path: &str, depth: Option<u32>, json: bool) -> Result<String> {
    let mut args: Vec<String> = vec!["get".into(), path.into(), doc_path.into()];
    if let Some(d) = depth {
        args.push("--depth".into());
        args.push(d.to_string());
    }
    if json {
        args.push("--json".into());
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Query a document with a CSS-like selector.
pub fn query(path: &str, selector: &str, json: bool) -> Result<String> {
    let mut args: Vec<String> = vec!["query".into(), path.into(), selector.into()];
    if json {
        args.push("--json".into());
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Set properties on a document element.
pub fn set(path: &str, doc_path: &str, props: &[(&str, &str)]) -> Result<String> {
    let mut args: Vec<String> = vec!["set".into(), path.into(), doc_path.into()];
    for (k, v) in props {
        args.push("--prop".into());
        args.push(format!("{}={}", k, v));
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Add an element to a document.
pub fn add(path: &str, parent: &str, element_type: &str, props: &[(&str, &str)]) -> Result<String> {
    let mut args: Vec<String> = vec![
        "add".into(),
        path.into(),
        parent.into(),
        "--type".into(),
        element_type.into(),
    ];
    for (k, v) in props {
        args.push("--prop".into());
        args.push(format!("{}={}", k, v));
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Add an element after a matched anchor.
pub fn add_after(
    path: &str,
    parent: &str,
    element_type: &str,
    anchor_find: &str,
    props: &[(&str, &str)],
) -> Result<String> {
    let mut args: Vec<String> = vec![
        "add".into(),
        path.into(),
        parent.into(),
        "--type".into(),
        element_type.into(),
        "--after".into(),
        format!("find:{}", anchor_find),
    ];
    for (k, v) in props {
        args.push("--prop".into());
        args.push(format!("{}={}", k, v));
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Replace matched text in a document.
pub fn replace(
    path: &str,
    doc_path: &str,
    find: &str,
    replacement: &str,
    regex: bool,
) -> Result<String> {
    let mut args: Vec<String> = vec!["set".into(), path.into()];
    if doc_path != "/" {
        args.push(doc_path.into());
    }
    args.push("--prop".into());
    args.push(format!("find={}", find));
    args.push("--prop".into());
    args.push(format!("replace={}", replacement));
    if regex {
        args.push("--prop".into());
        args.push("regex=true".into());
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Format matched text.
pub fn format_text(
    path: &str,
    doc_path: &str,
    find: &str,
    props: &[(&str, &str)],
    regex: bool,
) -> Result<String> {
    let mut args: Vec<String> = vec![
        "set".into(),
        path.into(),
        doc_path.into(),
        "--prop".into(),
        format!("find={}", find),
    ];
    if regex {
        args.push("--prop".into());
        args.push("regex=true".into());
    }
    for (k, v) in props {
        args.push("--prop".into());
        args.push(format!("{}={}", k, v));
    }
    let cmd_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&cmd_args)
}

/// Remove an element from a document.
pub fn remove(path: &str, doc_path: &str) -> Result<String> {
    run(&["remove", path, doc_path])
}

/// Open a document in resident mode (keeps file in memory, avoids lock conflicts).
pub fn open(path: &str) -> Result<String> {
    run(&["open", path])
}

/// Close a document (flushes changes).
pub fn close(path: &str) -> Result<String> {
    run(&["close", path])
}

/// Export document as HTML.
pub fn export_html(path: &str, browser: bool) -> Result<String> {
    let mut args = vec!["view", path, "html"];
    if browser {
        args.push("--browser");
    }
    run(&args)
}

/// Run batch operations from a JSON string of commands.
pub fn batch(path: &str, commands: &str, json: bool) -> Result<String> {
    let mut args = vec!["batch", path];
    if json {
        args.push("--json");
    }
    run_with_stdin(&args, commands)
}

// =============================================================================
// Typed response structs
// =============================================================================

/// Document statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliStats {
    pub paragraphs: Option<u32>,
    pub words: Option<u32>,
    pub pages: Option<u32>,
    pub slides: Option<u32>,
    pub shapes: Option<u32>,
    pub sheets: Option<u32>,
    pub rows: Option<u32>,
    pub cols: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Design suggestion for a slide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSuggestion {
    pub path: String,
    pub shape_type: String,
    pub suggestion: String,
}

/// Parse view stats output as CliStats.
pub fn parse_stats(text: &str) -> CliStats {
    let mut stats = CliStats {
        paragraphs: None,
        words: None,
        pages: None,
        slides: None,
        shapes: None,
        sheets: None,
        rows: None,
        cols: None,
    };

    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].trim();
        let val: u32 = parts[1].trim().parse().unwrap_or(0);
        match key {
            "Paragraphs" | "paragraphs" => stats.paragraphs = Some(val),
            "Words" | "words" => stats.words = Some(val),
            "Pages" | "pages" => stats.pages = Some(val),
            "Slides" | "slides" => stats.slides = Some(val),
            "Shapes" | "shapes" => stats.shapes = Some(val),
            "Sheets" | "sheets" => stats.sheets = Some(val),
            "Rows" | "rows" => stats.rows = Some(val),
            "Cols" | "cols" => stats.cols = Some(val),
            _ => {}
        }
    }

    stats
}

// =============================================================================
// High-level helpers
// =============================================================================

/// Insert text into a Word document.
pub fn docx_insert_paragraph(
    path: &str,
    parent: &str,
    text: &str,
    style: Option<&str>,
) -> Result<String> {
    let mut props = vec![("text", text)];
    if let Some(s) = style {
        props.push(("style", s));
    }
    add(path, parent, "paragraph", &props)
}

/// Make matched text bold in a Word document.
pub fn docx_bold(path: &str, find: &str) -> Result<String> {
    format_text(path, "/body", find, &[("bold", "true")], false)
}

/// Insert a cell value in an Excel spreadsheet.
pub fn xlsx_set_cell(path: &str, cell: &str, value: &str) -> Result<String> {
    set(path, cell, &[("value", value)])
}

/// Insert a formula in an Excel spreadsheet.
pub fn xlsx_set_formula(path: &str, cell: &str, formula: &str) -> Result<String> {
    set(path, cell, &[("value", formula)])
}

/// Read a cell range from Excel.
pub fn xlsx_get_range(path: &str, sheet: &str, range: &str, json: bool) -> Result<String> {
    get(path, &format!("/{}/{}", sheet, range), None, json)
}

/// Add a slide to a PowerPoint presentation.
pub fn pptx_add_slide(path: &str, layout: Option<&str>) -> Result<String> {
    let mut props = Vec::new();
    if let Some(l) = layout {
        props.push(("layout", l));
    }
    add(path, "/", "slide", &props)
}

/// Add a textbox to a slide.
pub fn pptx_add_textbox(
    path: &str,
    slide_path: &str,
    text: &str,
    x: Option<&str>,
    y: Option<&str>,
) -> Result<String> {
    let mut props = vec![("text", text)];
    if let Some(v) = x {
        props.push(("x", v));
    }
    if let Some(v) = y {
        props.push(("y", v));
    }
    add(path, slide_path, "shape", &props)
}

/// Set a shape property in PowerPoint.
pub fn pptx_set_shape(path: &str, shape_path: &str, props: &[(&str, &str)]) -> Result<String> {
    set(path, shape_path, props)
}

/// Get all shapes on a slide.
pub fn pptx_get_shapes(path: &str, slide_index: u32, json: bool) -> Result<String> {
    get(path, &format!("/slide[{}]", slide_index), Some(1), json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_type_from_extension() {
        assert_eq!(DocType::from_extension("doc.docx"), Some(DocType::Docx));
        assert_eq!(DocType::from_extension("sheet.xlsx"), Some(DocType::Xlsx));
        assert_eq!(DocType::from_extension("deck.pptx"), Some(DocType::Pptx));
        assert_eq!(DocType::from_extension("unknown.txt"), None);
    }

    #[test]
    fn cli_output_fields() {
        let out = CliOutput {
            success: true,
            stdout: "ok".to_string(),
            stderr: String::new(),
            exit_code: 0,
        };
        assert!(out.success);
        assert_eq!(out.exit_code, 0);
    }

    #[test]
    fn parse_stats_lines() {
        let text = "Paragraphs: 10\nWords: 500\nSlides: 5\n";
        let stats = parse_stats(text);
        assert_eq!(stats.paragraphs, Some(10));
        assert_eq!(stats.words, Some(500));
        assert_eq!(stats.slides, Some(5));
    }
}
