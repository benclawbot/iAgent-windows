use super::{Tool, ToolContext, ToolExecutionMode, ToolOutput};
use crate::safety::{PermissionRequest, PermissionResult, SafetySystem, Urgency, new_request_id};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct ComputerTool;

impl ComputerTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ComputerAction {
    Screenshot,
    Click,
    Type,
    Hotkey,
    Scroll,
    Wait,
    ActiveWindow,
    Context,
    OpenApp,
    ListApps,
}

impl ComputerAction {
    fn as_str(self) -> &'static str {
        match self {
            ComputerAction::Screenshot => "screenshot",
            ComputerAction::Click => "click",
            ComputerAction::Type => "type",
            ComputerAction::Hotkey => "hotkey",
            ComputerAction::Scroll => "scroll",
            ComputerAction::Wait => "wait",
            ComputerAction::ActiveWindow => "active_window",
            ComputerAction::Context => "context",
            ComputerAction::OpenApp => "open_app",
            ComputerAction::ListApps => "list_apps",
        }
    }

    fn requires_permission(self) -> bool {
        matches!(
            self,
            ComputerAction::Click
                | ComputerAction::Type
                | ComputerAction::Hotkey
                | ComputerAction::Scroll
        )
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    fn as_str(self) -> &'static str {
        match self {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
        }
    }
}

#[derive(Debug, Deserialize)]
struct ComputerInput {
    action: ComputerAction,
    x: Option<i32>,
    y: Option<i32>,
    button: Option<MouseButton>,
    text: Option<String>,
    keys: Option<Vec<String>>,
    amount: Option<i32>,
    duration_ms: Option<u64>,
    app: Option<String>,
}

#[async_trait]
impl Tool for ComputerTool {
    fn name(&self) -> &str {
        "computer"
    }

    fn description(&self) -> &str {
        concat!(
            "Native computer-use ACI for Windows desktop control. ",
            "Use only the constrained actions in the schema: screenshot, click, type, hotkey, ",
            "scroll, wait, active_window, context, open_app, and list_apps. Use open_app for ",
            "installed Desktop or Start Menu applications instead of shell, Python, browser, ",
            "or arbitrary code execution."
        )
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["screenshot", "click", "type", "hotkey", "scroll", "wait", "active_window", "context", "open_app", "list_apps"],
                    "description": "Constrained desktop action to execute."
                },
                "x": {
                    "type": "integer",
                    "description": "Screen x coordinate for click or optional scroll positioning."
                },
                "y": {
                    "type": "integer",
                    "description": "Screen y coordinate for click or optional scroll positioning."
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "description": "Mouse button for click. Defaults to left."
                },
                "text": {
                    "type": "string",
                    "description": "Text to type for action='type'."
                },
                "keys": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Keys for action='hotkey', for example ['ctrl','l'] or ['alt','tab']."
                },
                "amount": {
                    "type": "integer",
                    "description": "Scroll wheel notches for action='scroll'. Positive scrolls up, negative scrolls down."
                },
                "duration_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 30000,
                    "description": "Delay for action='wait'. Defaults to 1000ms and is capped at 30000ms."
                },
                "app": {
                    "type": "string",
                    "description": "Installed app or desktop shortcut name for open_app, or optional search text for list_apps. Prefer the user's visible app name, such as 'Hermes' or 'GanttMaker'."
                },
                "intent": super::intent_schema_property()
            },
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolOutput> {
        let params: ComputerInput = serde_json::from_value(input)?;
        if params.action.requires_permission()
            && matches!(ctx.execution_mode, ToolExecutionMode::AgentTurn)
        {
            return Ok(queue_computer_permission(&params, &ctx));
        }

        match params.action {
            ComputerAction::Wait => {
                let duration_ms = params.duration_ms.unwrap_or(1000).min(30_000);
                tokio::time::sleep(Duration::from_millis(duration_ms)).await;
                Ok(
                    ToolOutput::new(format!("Waited {duration_ms}ms.").to_string())
                        .with_title("computer: wait")
                        .with_metadata(json!({ "action": "wait", "duration_ms": duration_ms })),
                )
            }
            ComputerAction::Screenshot => capture_screenshot().await,
            ComputerAction::Click => {
                let x = params
                    .x
                    .ok_or_else(|| anyhow!("x is required for action='click'"))?;
                let y = params
                    .y
                    .ok_or_else(|| anyhow!("y is required for action='click'"))?;
                click(x, y, params.button.unwrap_or(MouseButton::Left)).await
            }
            ComputerAction::Type => {
                let text = params
                    .text
                    .ok_or_else(|| anyhow!("text is required for action='type'"))?;
                type_text(text).await
            }
            ComputerAction::Hotkey => {
                let keys = params
                    .keys
                    .ok_or_else(|| anyhow!("keys is required for action='hotkey'"))?;
                hotkey(keys).await
            }
            ComputerAction::Scroll => {
                let amount = params.amount.unwrap_or(-3);
                scroll(params.x, params.y, amount).await
            }
            ComputerAction::ActiveWindow => active_window(false).await,
            ComputerAction::Context => active_window(true).await,
            ComputerAction::OpenApp => {
                let app = params
                    .app
                    .ok_or_else(|| anyhow!("app is required for action='open_app'"))?;
                open_app(app).await
            }
            ComputerAction::ListApps => list_apps(params.app).await,
        }
    }
}

fn queue_computer_permission(params: &ComputerInput, ctx: &ToolContext) -> ToolOutput {
    let request_id = new_request_id();
    let action_name = format!("computer.{}", params.action.as_str());
    let description = permission_description(params);
    let request = PermissionRequest {
        id: request_id.clone(),
        action: action_name.clone(),
        description: description.clone(),
        rationale: concat!(
            "Desktop control mutates the user's active Windows session. ",
            "Agent-S style plans must cross iAgent's constrained computer tool ",
            "and permission boundary before mouse, keyboard, or scroll input is sent."
        )
        .to_string(),
        urgency: Urgency::Normal,
        wait: false,
        created_at: Utc::now(),
        context: Some(json!({
            "tool": "computer",
            "computer_action": params.action.as_str(),
            "session_id": ctx.session_id,
            "message_id": ctx.message_id,
            "tool_call_id": ctx.tool_call_id,
            "details": permission_details(params),
            "agent_s_contract": {
                "planner": "Agent-S-compatible ACI proposal",
                "executor": "iAgent Rust computer tool",
                "blocked_paths": ["shell", "python", "arbitrary code execution"]
            }
        })),
    };

    let result = SafetySystem::new().request_permission(request);
    let output = match result {
        PermissionResult::Queued { request_id } => format!(
            "Permission request queued (id: {request_id}). Action '{action_name}' is pending user review and was not executed."
        ),
        PermissionResult::Approved { message } => format!(
            "Permission approved but this computer action was not replayed automatically: {}",
            message.as_deref().unwrap_or("no message")
        ),
        PermissionResult::Denied { reason } => format!(
            "Permission denied. Action '{action_name}' was not executed: {}",
            reason.as_deref().unwrap_or("no reason given")
        ),
        PermissionResult::Timeout => {
            format!("Permission request timed out. Action '{action_name}' was not executed.")
        }
    };

    ToolOutput::new(output)
        .with_title(format!(
            "computer: permission required ({})",
            params.action.as_str()
        ))
        .with_metadata(json!({
            "action": params.action.as_str(),
            "permission_required": true,
            "permission_request_id": request_id,
            "executed": false
        }))
}

fn permission_description(params: &ComputerInput) -> String {
    match params.action {
        ComputerAction::Click => format!(
            "Click the {} mouse button at ({}, {}).",
            params.button.unwrap_or(MouseButton::Left).as_str(),
            params
                .x
                .map(|x| x.to_string())
                .unwrap_or_else(|| "unknown x".to_string()),
            params
                .y
                .map(|y| y.to_string())
                .unwrap_or_else(|| "unknown y".to_string())
        ),
        ComputerAction::Type => format!(
            "Type {} characters into the active desktop window.",
            params.text.as_deref().unwrap_or_default().chars().count()
        ),
        ComputerAction::Hotkey => format!(
            "Press hotkey {}.",
            params
                .keys
                .as_ref()
                .map(|keys| keys.join("+"))
                .unwrap_or_else(|| "<missing keys>".to_string())
        ),
        ComputerAction::Scroll => format!(
            "Scroll by {} wheel steps{}.",
            params.amount.unwrap_or(-3),
            match (params.x, params.y) {
                (Some(x), Some(y)) => format!(" at ({x}, {y})"),
                _ => String::new(),
            }
        ),
        ComputerAction::OpenApp => format!(
            "Open installed Desktop or Start Menu app '{}'.",
            params.app.as_deref().unwrap_or("<missing app>")
        ),
        _ => format!("Run computer action '{}'.", params.action.as_str()),
    }
}

fn permission_details(params: &ComputerInput) -> Value {
    json!({
        "x": params.x,
        "y": params.y,
        "button": params.button.map(MouseButton::as_str),
        "keys": params.keys,
        "amount": params.amount,
        "duration_ms": params.duration_ms,
        "app": params.app,
        "text_preview": params.text.as_deref().map(|text| preview_text(text, 200)),
        "text_char_count": params.text.as_deref().map(|text| text.chars().count()),
    })
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let mut preview: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

async fn open_app(app: String) -> Result<ToolOutput> {
    let app = app.trim().to_string();
    if app.is_empty() {
        return Err(anyhow!("app must not be empty for action='open_app'"));
    }

    #[cfg(windows)]
    {
        tokio::task::spawn_blocking(move || open_app_windows(&app))
            .await
            .context("open_app task failed")?
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        Err(anyhow!("computer open_app is only available on Windows"))
    }
}

async fn list_apps(filter: Option<String>) -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        tokio::task::spawn_blocking(move || list_apps_windows(filter))
            .await
            .context("list_apps task failed")?
    }

    #[cfg(not(windows))]
    {
        let _ = filter;
        Err(anyhow!("computer list_apps is only available on Windows"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppCandidate {
    name: String,
    path: PathBuf,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppMatch {
    candidate: AppCandidate,
    score: u8,
}

fn normalize_app_name(name: &str) -> String {
    name.chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn score_app_match(query: &str, candidate: &str) -> Option<u8> {
    let raw_query = query;
    let query = normalize_app_name(query);
    let candidate = normalize_app_name(candidate);
    if query.is_empty() || candidate.is_empty() {
        return None;
    }
    if candidate == query {
        return Some(100);
    }
    if candidate.starts_with(&query) {
        return Some(90);
    }
    if candidate.contains(&query) {
        return Some(80);
    }

    let raw_tokens: Vec<String> = raw_query
        .split_whitespace()
        .map(normalize_app_name)
        .filter(|token| !token.is_empty())
        .collect();
    if !raw_tokens.is_empty() && raw_tokens.iter().all(|token| candidate.contains(token)) {
        return Some(70);
    }

    None
}

fn find_app_matches(query: &str, candidates: &[AppCandidate], limit: usize) -> Vec<AppMatch> {
    let mut matches: Vec<AppMatch> = candidates
        .iter()
        .filter_map(|candidate| {
            score_app_match(query, &candidate.name).map(|score| AppMatch {
                candidate: candidate.clone(),
                score,
            })
        })
        .collect();

    matches.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| source_rank(&a.candidate.source).cmp(&source_rank(&b.candidate.source)))
            .then_with(|| a.candidate.name.cmp(&b.candidate.name))
            .then_with(|| a.candidate.path.cmp(&b.candidate.path))
    });
    matches.truncate(limit);
    matches
}

fn source_rank(source: &str) -> u8 {
    match source {
        "Desktop" => 0,
        "OneDrive Desktop" => 1,
        "Public Desktop" => 2,
        "Start Menu" => 3,
        "Common Start Menu" => 4,
        _ => 5,
    }
}

#[cfg(windows)]
fn open_app_windows(app: &str) -> Result<ToolOutput> {
    let candidates = discover_app_candidates()?;
    let matches = find_app_matches(app, &candidates, 8);
    let best = matches
        .first()
        .ok_or_else(|| no_app_match_error(app, &candidates))?;

    if matches.len() > 1
        && matches[0].score < 100
        && matches[0].score == matches[1].score
        && source_rank(&matches[0].candidate.source) == source_rank(&matches[1].candidate.source)
    {
        let names = matches
            .iter()
            .take(5)
            .map(|m| {
                format!(
                    "{} ({}, {})",
                    m.candidate.name,
                    m.candidate.source,
                    m.candidate.path.display()
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow!(
            "multiple installed apps match '{app}': {names}. Use action='list_apps' with app='{app}' and then retry with the exact app name."
        ));
    }

    shell_execute_path(&best.candidate.path)?;
    Ok(ToolOutput::new(format!(
        "Opened app '{}' from {}: {}",
        best.candidate.name,
        best.candidate.source,
        best.candidate.path.display()
    ))
    .with_title(format!("computer: open_app {}", best.candidate.name))
    .with_metadata(json!({
        "action": "open_app",
        "app": app,
        "matched_name": best.candidate.name,
        "matched_path": best.candidate.path,
        "source": best.candidate.source,
        "executed": true
    })))
}

#[cfg(windows)]
fn list_apps_windows(filter: Option<String>) -> Result<ToolOutput> {
    let mut candidates = discover_app_candidates()?;
    candidates.sort_by(|a, b| {
        source_rank(&a.source)
            .cmp(&source_rank(&b.source))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });

    let total = candidates.len();
    let matches: Vec<AppCandidate> = if let Some(ref query) = filter {
        find_app_matches(query, &candidates, 50)
            .into_iter()
            .map(|m| m.candidate)
            .collect()
    } else {
        candidates.into_iter().take(50).collect()
    };

    let header = match filter.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(query) => format!("Found {} app match(es) for '{}'.", matches.len(), query),
        None => {
            format!("Found {total} installed Desktop/Start Menu app shortcut(s). Showing first 50.")
        }
    };
    let lines = matches
        .iter()
        .map(|candidate| {
            format!(
                "- {} [{}] {}",
                candidate.name,
                candidate.source,
                candidate.path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let output = if lines.is_empty() {
        header
    } else {
        format!("{header}\n{lines}")
    };

    Ok(ToolOutput::new(output)
        .with_title("computer: list_apps")
        .with_metadata(json!({
            "action": "list_apps",
            "filter": filter,
            "count": matches.len(),
            "total": total
        })))
}

#[cfg(windows)]
fn no_app_match_error(app: &str, candidates: &[AppCandidate]) -> anyhow::Error {
    let sample = candidates
        .iter()
        .take(12)
        .map(|candidate| candidate.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    anyhow!(
        "no Desktop or Start Menu app matched '{app}'. Use action='list_apps' to inspect available apps. Sample apps: {sample}"
    )
}

#[cfg(windows)]
fn discover_app_candidates() -> Result<Vec<AppCandidate>> {
    let mut candidates = Vec::new();
    for (root, source) in app_search_roots() {
        collect_app_candidates(&root, &source, &mut candidates)?;
    }

    candidates.sort_by(|a, b| {
        source_rank(&a.source)
            .cmp(&source_rank(&b.source))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });
    candidates.dedup_by(|a, b| a.name == b.name && a.path == b.path);
    Ok(candidates)
}

#[cfg(windows)]
fn app_search_roots() -> Vec<(PathBuf, String)> {
    let mut roots = Vec::new();
    if let Some(user_profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        roots.push((user_profile.join("Desktop"), "Desktop".to_string()));
    }
    for var in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
        if let Some(one_drive) = std::env::var_os(var).map(PathBuf::from) {
            roots.push((one_drive.join("Desktop"), "OneDrive Desktop".to_string()));
        }
    }
    if let Some(public) = std::env::var_os("PUBLIC").map(PathBuf::from) {
        roots.push((public.join("Desktop"), "Public Desktop".to_string()));
    }
    if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
        roots.push((
            appdata.join("Microsoft\\Windows\\Start Menu\\Programs"),
            "Start Menu".to_string(),
        ));
    }
    if let Some(programdata) = std::env::var_os("PROGRAMDATA").map(PathBuf::from) {
        roots.push((
            programdata.join("Microsoft\\Windows\\Start Menu\\Programs"),
            "Common Start Menu".to_string(),
        ));
    }

    let mut unique = Vec::new();
    for root in roots {
        if root.0.exists()
            && !unique
                .iter()
                .any(|(path, _): &(PathBuf, String)| *path == root.0)
        {
            unique.push(root);
        }
    }
    unique
}

#[cfg(windows)]
fn collect_app_candidates(root: &Path, source: &str, out: &mut Vec<AppCandidate>) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !is_launchable_app_path(&path) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            out.push(AppCandidate {
                name: stem.to_string(),
                path,
                source: source.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_launchable_app_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "lnk" | "appref-ms" | "exe"
    )
}

#[cfg(windows)]
fn shell_execute_path(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let file: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    if (result as isize) <= 32 {
        return Err(anyhow!(
            "ShellExecuteW failed with code {} for {}",
            result as isize,
            path.display()
        ));
    }
    Ok(())
}

async fn capture_screenshot() -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        let result = tokio::task::spawn_blocking(capture_screenshot_windows)
            .await
            .context("screenshot task failed")??;
        let bytes = tokio::fs::read(&result.path)
            .await
            .with_context(|| format!("failed to read screenshot {}", result.path.display()))?;
        let _ = tokio::fs::remove_file(&result.path).await;
        Ok(ToolOutput::new(format!(
            "Captured desktop screenshot ({}x{}).",
            result.width, result.height
        ))
        .with_title("computer: screenshot")
        .with_metadata(json!({
            "action": "screenshot",
            "width": result.width,
            "height": result.height,
            "origin": { "x": result.x, "y": result.y }
        }))
        .with_labeled_image(
            "image/png",
            STANDARD.encode(&bytes),
            format!("desktop screenshot: {}x{}", result.width, result.height),
        ))
    }
    #[cfg(not(windows))]
    {
        Err(anyhow!("computer screenshot is only available on Windows"))
    }
}

async fn click(x: i32, y: i32, button: MouseButton) -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        tokio::task::spawn_blocking(move || click_windows(x, y, button))
            .await
            .context("click task failed")??;
        Ok(ToolOutput::new(format!("Clicked at ({x}, {y})."))
            .with_title("computer: click")
            .with_metadata(json!({ "action": "click", "x": x, "y": y })))
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y, button);
        Err(anyhow!("computer click is only available on Windows"))
    }
}

async fn type_text(text: String) -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        let char_count = text.chars().count();
        tokio::task::spawn_blocking(move || type_text_windows(&text))
            .await
            .context("type task failed")??;
        Ok(ToolOutput::new(format!("Typed {char_count} characters."))
            .with_title("computer: type")
            .with_metadata(json!({ "action": "type", "characters": char_count })))
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        Err(anyhow!("computer type is only available on Windows"))
    }
}

async fn hotkey(keys: Vec<String>) -> Result<ToolOutput> {
    if keys.is_empty() {
        return Err(anyhow!("keys must not be empty for action='hotkey'"));
    }
    #[cfg(windows)]
    {
        let rendered = keys.join("+");
        tokio::task::spawn_blocking(move || hotkey_windows(&keys))
            .await
            .context("hotkey task failed")??;
        Ok(ToolOutput::new(format!("Pressed hotkey {rendered}."))
            .with_title("computer: hotkey")
            .with_metadata(json!({ "action": "hotkey", "keys": rendered })))
    }
    #[cfg(not(windows))]
    {
        let _ = keys;
        Err(anyhow!("computer hotkey is only available on Windows"))
    }
}

async fn scroll(x: Option<i32>, y: Option<i32>, amount: i32) -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        tokio::task::spawn_blocking(move || scroll_windows(x, y, amount))
            .await
            .context("scroll task failed")??;
        Ok(ToolOutput::new(format!("Scrolled {amount} wheel notches."))
            .with_title("computer: scroll")
            .with_metadata(json!({ "action": "scroll", "x": x, "y": y, "amount": amount })))
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y, amount);
        Err(anyhow!("computer scroll is only available on Windows"))
    }
}

async fn active_window(include_context: bool) -> Result<ToolOutput> {
    #[cfg(windows)]
    {
        let info = tokio::task::spawn_blocking(move || active_window_windows(include_context))
            .await
            .context("active window task failed")??;
        Ok(info)
    }
    #[cfg(not(windows))]
    {
        let _ = include_context;
        Err(anyhow!(
            "computer active_window is only available on Windows"
        ))
    }
}

#[cfg(windows)]
struct ScreenshotResult {
    path: std::path::PathBuf,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[cfg(windows)]
fn capture_screenshot_windows() -> Result<ScreenshotResult> {
    use image::ColorType;
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, RGBQUAD,
        ReleaseDC, SRCCOPY, SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if width <= 0 || height <= 0 {
            return Err(anyhow!("invalid virtual screen size {width}x{height}"));
        }

        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return Err(anyhow!("GetDC failed"));
        }

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err(anyhow!("CreateCompatibleDC failed"));
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err(anyhow!("CreateCompatibleBitmap failed"));
        }

        let old = SelectObject(mem_dc, bitmap as _);
        let blit_ok = BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        );
        if blit_ok == 0 {
            SelectObject(mem_dc, old);
            DeleteObject(bitmap as _);
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err(anyhow!("BitBlt failed"));
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        let mut bgra = vec![0u8; (width as usize) * (height as usize) * 4];
        let rows = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            bgra.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old);
        DeleteObject(bitmap as _);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);

        if rows == 0 {
            return Err(anyhow!("GetDIBits failed"));
        }

        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }

        let path = temp_computer_path("screenshot", "png");
        image::save_buffer(&path, &bgra, width as u32, height as u32, ColorType::Rgba8)
            .with_context(|| format!("failed to save screenshot {}", path.display()))?;

        Ok(ScreenshotResult {
            path,
            x,
            y,
            width,
            height,
        })
    }
}

#[cfg(windows)]
fn click_windows(x: i32, y: i32, button: MouseButton) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
        MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEINPUT, SendInput,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

    let (down, up) = match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    };
    unsafe {
        if SetCursorPos(x, y) == 0 {
            return Err(anyhow!("SetCursorPos failed"));
        }
        send_inputs(&[mouse_input(down, 0), mouse_input(up, 0)])?;
    }
    Ok(())
}

#[cfg(windows)]
fn scroll_windows(x: Option<i32>, y: Option<i32>, amount: i32) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::MOUSEEVENTF_WHEEL;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

    if let (Some(x), Some(y)) = (x, y) {
        unsafe {
            if SetCursorPos(x, y) == 0 {
                return Err(anyhow!("SetCursorPos failed"));
            }
        }
    }
    let wheel_delta = amount.saturating_mul(120) as u32;
    unsafe {
        send_inputs(&[mouse_input(MOUSEEVENTF_WHEEL, wheel_delta)])?;
    }
    Ok(())
}

#[cfg(windows)]
fn type_text_windows(text: &str) -> Result<()> {
    let mut inputs = Vec::new();
    for unit in text.encode_utf16() {
        inputs.push(unicode_key_input(unit, false));
        inputs.push(unicode_key_input(unit, true));
    }
    unsafe {
        send_inputs(&inputs)?;
    }
    Ok(())
}

#[cfg(windows)]
fn hotkey_windows(keys: &[String]) -> Result<()> {
    let mut vks = Vec::with_capacity(keys.len());
    for key in keys {
        vks.push(parse_virtual_key(key)?);
    }

    let mut inputs = Vec::with_capacity(vks.len() * 2);
    for vk in &vks {
        inputs.push(virtual_key_input(*vk, false));
    }
    for vk in vks.iter().rev() {
        inputs.push(virtual_key_input(*vk, true));
    }
    unsafe {
        send_inputs(&inputs)?;
    }
    Ok(())
}

#[cfg(windows)]
fn active_window_windows(include_context: bool) -> Result<ToolOutput> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowRect, GetWindowTextW,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return Err(anyhow!("GetForegroundWindow returned no active window"));
        }

        let mut title_buf = vec![0u16; 512];
        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
        let title = String::from_utf16_lossy(&title_buf[..len as usize]);

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let has_rect = GetWindowRect(hwnd, &mut rect) != 0;

        let mut metadata = json!({
            "action": if include_context { "context" } else { "active_window" },
            "hwnd": hwnd as isize,
            "title": title,
        });
        if has_rect {
            metadata["bounds"] = json!({
                "left": rect.left,
                "top": rect.top,
                "right": rect.right,
                "bottom": rect.bottom,
                "width": rect.right - rect.left,
                "height": rect.bottom - rect.top,
            });
        }

        if include_context {
            let mut cursor = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut cursor) != 0 {
                metadata["cursor"] = json!({ "x": cursor.x, "y": cursor.y });
            }
            metadata["screen"] = json!({
                "x": GetSystemMetrics(SM_XVIRTUALSCREEN),
                "y": GetSystemMetrics(SM_YVIRTUALSCREEN),
                "width": GetSystemMetrics(SM_CXVIRTUALSCREEN),
                "height": GetSystemMetrics(SM_CYVIRTUALSCREEN),
            });
        }

        let mut lines = vec![format!("Active window: {title}")];
        if has_rect {
            lines.push(format!(
                "Bounds: left={}, top={}, width={}, height={}",
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top
            ));
        }
        Ok(ToolOutput::new(lines.join("\n"))
            .with_title(if include_context {
                "computer: context"
            } else {
                "computer: active_window"
            })
            .with_metadata(metadata))
    }
}

#[cfg(windows)]
unsafe fn send_inputs(
    inputs: &[windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT],
) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput};

    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(anyhow!("SendInput sent {sent} of {} inputs", inputs.len()));
    }
    Ok(())
}

#[cfg(windows)]
fn mouse_input(
    flags: windows_sys::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
    mouse_data: u32,
) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_MOUSE, MOUSEINPUT,
    };

    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: mouse_data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn unicode_key_input(
    unit: u16,
    key_up: bool,
) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: unit,
                dwFlags: KEYEVENTF_UNICODE | if key_up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn virtual_key_input(
    vk: windows_sys::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
    key_up: bool,
) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn parse_virtual_key(
    key: &str,
) -> Result<windows_sys::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    let normalized = key.trim().to_ascii_lowercase();
    let vk = match normalized.as_str() {
        "ctrl" | "control" => VK_CONTROL,
        "shift" => VK_SHIFT,
        "alt" | "option" => VK_MENU,
        "win" | "windows" | "meta" | "super" => VK_LWIN,
        "enter" | "return" => VK_RETURN,
        "tab" => VK_TAB,
        "esc" | "escape" => VK_ESCAPE,
        "space" => VK_SPACE,
        "backspace" => VK_BACK,
        "delete" | "del" => VK_DELETE,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" | "page_up" => VK_PRIOR,
        "pagedown" | "page_down" => VK_NEXT,
        "insert" | "ins" => VK_INSERT,
        "f1" => VK_F1,
        "f2" => VK_F2,
        "f3" => VK_F3,
        "f4" => VK_F4,
        "f5" => VK_F5,
        "f6" => VK_F6,
        "f7" => VK_F7,
        "f8" => VK_F8,
        "f9" => VK_F9,
        "f10" => VK_F10,
        "f11" => VK_F11,
        "f12" => VK_F12,
        single if single.len() == 1 => {
            let b = single.as_bytes()[0];
            if b.is_ascii_alphanumeric() {
                b.to_ascii_uppercase() as u16
            } else {
                return Err(anyhow!("unsupported hotkey key '{key}'"));
            }
        }
        _ => return Err(anyhow!("unsupported hotkey key '{key}'")),
    };
    Ok(vk)
}

#[cfg(windows)]
fn temp_computer_path(label: &str, ext: &str) -> std::path::PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("iagent-computer-{label}-{ts}.{ext}"))
}

#[cfg(test)]
#[path = "computer_tests.rs"]
mod computer_tests;
