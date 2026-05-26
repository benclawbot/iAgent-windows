use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result as AnyhowResult;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub mod hotkey;
pub use hotkey::{HotkeyEvent, HotkeyManager};
pub type DesktopMonitorResult<T> = AnyhowResult<T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextType {
    Email,
    Document,
    Presentation,
    Code,
    Chat,
    Browser,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowContext {
    pub app_name: String,
    pub window_title: String,
    pub context_type: ContextType,
    pub text_content: Option<String>,
    pub cursor_position: (i32, i32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawNotification {
    pub app: String,
    pub title: String,
    pub preview: String,
    pub sender: Option<String>,
    pub cc_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportantNotification {
    pub id: String,
    pub app: String,
    pub title: String,
    pub preview: String,
    pub importance: f32,
    pub state: NotificationState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationState {
    New,
    Updated,
    Dismissed,
}

#[derive(Debug, Clone, Default)]
pub struct UserPatterns {
    pub vip_senders: Vec<String>,
    pub urgent_keywords: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportanceScorer {
    patterns: UserPatterns,
}

impl ImportanceScorer {
    pub fn new(patterns: UserPatterns) -> Self {
        Self { patterns }
    }

    pub fn score(&self, notification: &RawNotification) -> f32 {
        let mut score: f32 = 0.0;
        let title = notification.title.to_ascii_lowercase();
        let preview = notification.preview.to_ascii_lowercase();

        if let Some(sender) = &notification.sender {
            let sender_lc = sender.to_ascii_lowercase();
            if self
                .patterns
                .vip_senders
                .iter()
                .any(|entry| sender_lc.contains(&entry.to_ascii_lowercase()))
            {
                score += 50.0;
            }
        }

        let keywords = if self.patterns.urgent_keywords.is_empty() {
            vec![
                "urgent".to_string(),
                "important".to_string(),
                "deadline".to_string(),
            ]
        } else {
            self.patterns.urgent_keywords.clone()
        };
        if keywords.iter().any(|kw| {
            let kw = kw.to_ascii_lowercase();
            title.contains(&kw) || preview.contains(&kw)
        }) {
            score += 30.0;
        }

        if title.contains("re:") || preview.contains("reply") {
            score += 20.0;
        }

        if notification.cc_only {
            score -= 20.0;
        }

        score.clamp(0.0, 100.0)
    }
}

pub struct NotificationDetector {
    scorer: ImportanceScorer,
    check_interval: Duration,
    pending: Arc<Mutex<VecDeque<RawNotification>>>,
    dedupe_window: Duration,
    throttle_window: Duration,
    stale_after: Duration,
    min_score: f32,
    redaction_policy: RedactionPolicy,
}

#[derive(Debug, Clone)]
struct ActiveNotification {
    raw: RawNotification,
    importance: f32,
    last_seen: Instant,
    last_emitted: Instant,
}

impl NotificationDetector {
    pub fn new(scorer: ImportanceScorer, check_interval: Duration) -> Self {
        Self {
            scorer,
            check_interval,
            pending: Arc::new(Mutex::new(VecDeque::new())),
            dedupe_window: Duration::from_secs(30),
            throttle_window: Duration::from_secs(10),
            stale_after: Duration::from_secs(90),
            min_score: 1.0,
            redaction_policy: RedactionPolicy::default(),
        }
    }

    pub fn with_dedupe_window(mut self, dedupe_window: Duration) -> Self {
        self.dedupe_window = dedupe_window;
        self
    }

    pub fn with_throttle_window(mut self, throttle_window: Duration) -> Self {
        self.throttle_window = throttle_window;
        self
    }

    pub fn with_stale_after(mut self, stale_after: Duration) -> Self {
        self.stale_after = stale_after;
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score;
        self
    }

    pub fn with_redaction_policy(mut self, redaction_policy: RedactionPolicy) -> Self {
        self.redaction_policy = redaction_policy;
        self
    }

    /// Push an externally observed notification into the detector queue.
    pub fn ingest_notification(&self, notification: RawNotification) {
        if let Ok(mut queue) = self.pending.lock() {
            queue.push_back(notification);
        }
    }

    fn drain_pending_notifications(
        pending: &Arc<Mutex<VecDeque<RawNotification>>>,
    ) -> Vec<RawNotification> {
        if let Ok(mut queue) = pending.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    pub async fn monitor_notifications(&self) -> mpsc::UnboundedReceiver<ImportantNotification> {
        let (tx, rx) = mpsc::unbounded_channel();
        let scorer = self.scorer.clone();
        let interval = self.check_interval;
        let dedupe_window = self.dedupe_window;
        let throttle_window = self.throttle_window;
        let stale_after = self.stale_after;
        let min_score = self.min_score;
        let redaction_policy = self.redaction_policy.clone();
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let mut active: HashMap<String, ActiveNotification> = HashMap::new();
            loop {
                let now = Instant::now();
                let mut notifications = Self::drain_pending_notifications(&pending);
                notifications.extend(platform::poll_notifications());
                let mut seen_keys = HashSet::new();

                for raw in notifications {
                    let raw = redact_notification(raw, &redaction_policy);
                    let key = notification_key(&raw);
                    seen_keys.insert(key.clone());
                    let importance = scorer.score(&raw);
                    if importance < min_score {
                        continue;
                    }

                    if let Some(existing) = active.get_mut(&key) {
                        let content_changed =
                            existing.raw.title != raw.title || existing.raw.preview != raw.preview;
                        existing.raw = raw.clone();
                        existing.importance = importance;
                        existing.last_seen = now;

                        if content_changed
                            && now.duration_since(existing.last_emitted) >= throttle_window
                        {
                            let important = ImportantNotification {
                                id: key.clone(),
                                app: raw.app.clone(),
                                title: raw.title.clone(),
                                preview: raw.preview.clone(),
                                importance,
                                state: NotificationState::Updated,
                            };
                            if tx.send(important).is_err() {
                                return;
                            }
                            existing.last_emitted = now;
                        }
                        continue;
                    }

                    let should_emit_new = match active.get(&key) {
                        Some(existing) => {
                            now.duration_since(existing.last_emitted) >= dedupe_window
                        }
                        None => true,
                    };
                    if !should_emit_new {
                        continue;
                    }

                    let important = ImportantNotification {
                        id: key.clone(),
                        app: raw.app.clone(),
                        title: raw.title.clone(),
                        preview: raw.preview.clone(),
                        importance,
                        state: NotificationState::New,
                    };
                    if tx.send(important).is_err() {
                        return;
                    }
                    active.insert(
                        key,
                        ActiveNotification {
                            raw,
                            importance,
                            last_seen: now,
                            last_emitted: now,
                        },
                    );
                }

                let stale_keys: Vec<String> = active
                    .iter()
                    .filter_map(|(key, tracked)| {
                        if seen_keys.contains(key) {
                            return None;
                        }
                        if now.duration_since(tracked.last_seen) >= stale_after {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                for key in stale_keys {
                    if let Some(tracked) = active.remove(&key) {
                        let important = ImportantNotification {
                            id: key.clone(),
                            app: tracked.raw.app,
                            title: tracked.raw.title,
                            preview: tracked.raw.preview,
                            importance: tracked.importance,
                            state: NotificationState::Dismissed,
                        };
                        if tx.send(important).is_err() {
                            return;
                        }
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });
        rx
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RedactionPolicy {
    pub mask_email_addresses: bool,
    pub mask_api_tokens: bool,
    pub mask_potential_financial_numbers: bool,
}

impl Default for RedactionPolicy {
    fn default() -> Self {
        Self {
            mask_email_addresses: true,
            mask_api_tokens: true,
            mask_potential_financial_numbers: true,
        }
    }
}

fn compile_regex(pattern: &str) -> Option<Regex> {
    Regex::new(pattern).ok()
}

fn redact_text(text: &str, policy: &RedactionPolicy) -> String {
    let mut out = text.to_string();

    if policy.mask_api_tokens {
        static TOKEN_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
        let token_patterns = TOKEN_PATTERNS.get_or_init(|| {
            [
                r"sk-[A-Za-z0-9_-]{20,}",
                r"ghp_[A-Za-z0-9]{20,}",
                r"github_pat_[A-Za-z0-9_]{20,}",
                r"AKIA[0-9A-Z]{16}",
            ]
            .iter()
            .filter_map(|p| compile_regex(p))
            .collect()
        });
        for re in token_patterns {
            out = re.replace_all(&out, "[REDACTED_TOKEN]").into_owned();
        }
    }

    if policy.mask_email_addresses {
        static EMAIL_RE: OnceLock<Option<Regex>> = OnceLock::new();
        if let Some(re) = EMAIL_RE
            .get_or_init(|| compile_regex(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"))
            .as_ref()
        {
            out = re.replace_all(&out, "[REDACTED_EMAIL]").into_owned();
        }
    }

    if policy.mask_potential_financial_numbers {
        static CARD_RE: OnceLock<Option<Regex>> = OnceLock::new();
        if let Some(re) = CARD_RE
            .get_or_init(|| compile_regex(r"\b(?:\d[ -]*?){13,19}\b"))
            .as_ref()
        {
            out = re.replace_all(&out, "[REDACTED_NUMBER]").into_owned();
        }
    }

    out
}

fn redact_notification(raw: RawNotification, policy: &RedactionPolicy) -> RawNotification {
    RawNotification {
        app: raw.app,
        title: redact_text(&raw.title, policy),
        preview: redact_text(&raw.preview, policy),
        sender: raw.sender.map(|value| redact_text(&value, policy)),
        cc_only: raw.cc_only,
    }
}

fn notification_key(notification: &RawNotification) -> String {
    format!(
        "{}|{}|{}",
        notification.app.to_ascii_lowercase(),
        notification.title.to_ascii_lowercase(),
        notification
            .sender
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    )
}

#[derive(Debug, Clone)]
pub struct DesktopMonitorConfig {
    pub poll_interval: Duration,
    pub idle_debounce: Duration,
    pub min_text_len_delta: usize,
}

impl Default for DesktopMonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(500),
            idle_debounce: Duration::from_secs(2),
            min_text_len_delta: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesktopMonitor {
    config: DesktopMonitorConfig,
}

impl DesktopMonitor {
    pub fn new() -> DesktopMonitorResult<Self> {
        Self::with_config(DesktopMonitorConfig::default())
    }

    pub fn with_config(config: DesktopMonitorConfig) -> DesktopMonitorResult<Self> {
        platform::ensure_automation_available()?;
        Ok(Self { config })
    }

    pub async fn start_monitoring(&self) -> mpsc::UnboundedReceiver<WindowContext> {
        let (tx, rx) = mpsc::unbounded_channel();
        let config = Arc::new(self.config.clone());
        thread::spawn(move || run_monitor_loop(config, tx));
        rx
    }

    pub fn detect_context_type(process_name: &str, window_class: &str) -> ContextType {
        let app = process_name.to_ascii_lowercase();
        let class_name = window_class.to_ascii_lowercase();

        let app_leaf = Path::new(&app)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(app.as_str());

        match app_leaf {
            "outlook.exe" => ContextType::Email,
            "winword.exe" | "wordpad.exe" | "notepad.exe" => ContextType::Document,
            "powerpnt.exe" => ContextType::Presentation,
            "code.exe" | "devenv.exe" | "rider64.exe" => ContextType::Code,
            "slack.exe" | "teams.exe" | "discord.exe" => ContextType::Chat,
            "chrome.exe" | "firefox.exe" | "msedge.exe" => ContextType::Browser,
            _ => {
                if class_name.contains("chrome_widgetwin") {
                    ContextType::Browser
                } else if class_name.contains("outlook") {
                    ContextType::Email
                } else {
                    ContextType::Unknown
                }
            }
        }
    }
}

fn run_monitor_loop(config: Arc<DesktopMonitorConfig>, tx: mpsc::UnboundedSender<WindowContext>) {
    let _com_guard = platform::ComGuard::initialize_for_monitor_thread().ok();
    let _focus_hook = platform::install_focus_change_hook();
    let mut last_focus_event_epoch = platform::focus_event_epoch();

    let mut last_sent: Option<WindowContext> = None;
    let mut last_text_change: Option<Instant> = None;

    loop {
        let current_focus_event_epoch = platform::focus_event_epoch();
        let focus_event_changed = current_focus_event_epoch != last_focus_event_epoch;
        if focus_event_changed {
            last_focus_event_epoch = current_focus_event_epoch;
        }

        let current = match platform::capture_window_context() {
            Some(context) => context,
            None => {
                thread::sleep(config.poll_interval);
                continue;
            }
        };

        let should_emit = should_emit_update(
            &config,
            &last_sent,
            &current,
            &mut last_text_change,
            Instant::now(),
            focus_event_changed,
        );

        if should_emit {
            if tx.send(current.clone()).is_err() {
                break;
            }
            last_sent = Some(current);
        }

        thread::sleep(config.poll_interval);
    }
}

fn should_emit_update(
    config: &DesktopMonitorConfig,
    previous: &Option<WindowContext>,
    current: &WindowContext,
    last_text_change: &mut Option<Instant>,
    now: Instant,
    force_focus_emit: bool,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };

    if force_focus_emit {
        *last_text_change = None;
        return true;
    }

    let focus_changed =
        previous.app_name != current.app_name || previous.window_title != current.window_title;

    if focus_changed {
        *last_text_change = None;
        return true;
    }

    let previous_len = previous
        .text_content
        .as_deref()
        .map(|text| text.chars().count())
        .unwrap_or(0);
    let current_len = current
        .text_content
        .as_deref()
        .map(|text| text.chars().count())
        .unwrap_or(0);

    let len_delta = previous_len.abs_diff(current_len);
    if len_delta >= config.min_text_len_delta {
        *last_text_change = Some(now);
        return true;
    }

    let text_changed = previous.text_content != current.text_content;
    if text_changed {
        if last_text_change.is_none() {
            *last_text_change = Some(now);
        }
        if let Some(changed_at) = *last_text_change
            && now.duration_since(changed_at) >= config.idle_debounce
        {
            *last_text_change = None;
            return true;
        }
    } else {
        *last_text_change = None;
    }

    false
}

mod platform {
    #[cfg(windows)]
    mod windows_impl {
        use std::path::Path;
        use std::sync::atomic::{AtomicU64, Ordering};

        use windows::Win32::Foundation::{CloseHandle, HWND, POINT};
        use windows::Win32::System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        };
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        };
        use windows::Win32::UI::Accessibility::{
            CUIAutomation, HWINEVENTHOOK, IUIAutomation, IUIAutomationTextPattern,
            IUIAutomationValuePattern, SetWinEventHook, UIA_TextPatternId, UIA_ValuePatternId,
            UnhookWinEvent, WINEVENTPROC,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, GetClassNameW, GetCursorPos,
            GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, WINEVENT_OUTOFCONTEXT,
            WINEVENT_SKIPOWNPROCESS,
        };
        use windows::core::PWSTR;

        use crate::DesktopMonitorResult;
        use crate::{RawNotification, WindowContext};

        static FOCUS_EVENT_EPOCH: AtomicU64 = AtomicU64::new(0);

        pub struct ComGuard;
        pub struct FocusHookGuard {
            foreground_hook: Option<HWINEVENTHOOK>,
            object_focus_hook: Option<HWINEVENTHOOK>,
        }

        impl ComGuard {
            pub fn initialize_for_monitor_thread() -> DesktopMonitorResult<Self> {
                unsafe {
                    CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
                }
                Ok(Self)
            }
        }

        impl Drop for ComGuard {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }

        impl Drop for FocusHookGuard {
            fn drop(&mut self) {
                unsafe {
                    if let Some(hook) = self.foreground_hook.take()
                        && !hook.is_invalid()
                    {
                        let _ = UnhookWinEvent(hook);
                    }
                    if let Some(hook) = self.object_focus_hook.take()
                        && !hook.is_invalid()
                    {
                        let _ = UnhookWinEvent(hook);
                    }
                }
            }
        }

        pub fn ensure_automation_available() -> DesktopMonitorResult<()> {
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
                let _automation: IUIAutomation =
                    CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
                CoUninitialize();
            }
            Ok(())
        }

        pub fn install_focus_change_hook() -> Option<FocusHookGuard> {
            unsafe extern "system" fn on_focus_event(
                _hook: HWINEVENTHOOK,
                _event: u32,
                _hwnd: HWND,
                _idobject: i32,
                _idchild: i32,
                _ideventthread: u32,
                _dwmseventtime: u32,
            ) {
                FOCUS_EVENT_EPOCH.fetch_add(1, Ordering::Relaxed);
            }

            let callback: WINEVENTPROC = Some(on_focus_event);
            let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
            let foreground_hook = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    callback,
                    0,
                    0,
                    flags,
                )
            };
            let object_focus_hook = unsafe {
                SetWinEventHook(
                    EVENT_OBJECT_FOCUS,
                    EVENT_OBJECT_FOCUS,
                    None,
                    callback,
                    0,
                    0,
                    flags,
                )
            };

            let foreground_hook = if foreground_hook.is_invalid() {
                None
            } else {
                Some(foreground_hook)
            };
            let object_focus_hook = if object_focus_hook.is_invalid() {
                None
            } else {
                Some(object_focus_hook)
            };

            if foreground_hook.is_none() && object_focus_hook.is_none() {
                return None;
            }

            Some(FocusHookGuard {
                foreground_hook,
                object_focus_hook,
            })
        }

        pub fn focus_event_epoch() -> u64 {
            FOCUS_EVENT_EPOCH.load(Ordering::Relaxed)
        }

        pub fn capture_window_context() -> Option<WindowContext> {
            unsafe {
                let hwnd = GetForegroundWindow();
                if hwnd.0.is_null() {
                    return None;
                }

                let window_title = window_text(hwnd);
                let window_class = window_class(hwnd);
                let app_name =
                    process_name_for_window(hwnd).unwrap_or_else(|| "unknown".to_string());

                let mut cursor = POINT::default();
                let _ = GetCursorPos(&mut cursor);

                let text_content =
                    extract_text_from_focused_element().or_else(|| fallback_text(&window_title));
                let context_type =
                    super::super::DesktopMonitor::detect_context_type(&app_name, &window_class);

                Some(WindowContext {
                    app_name,
                    window_title,
                    context_type,
                    text_content,
                    cursor_position: (cursor.x, cursor.y),
                })
            }
        }

        pub fn poll_notifications() -> Vec<RawNotification> {
            let Some(context) = capture_window_context() else {
                return Vec::new();
            };

            let app = context.app_name.to_ascii_lowercase();
            let app_is_message_surface = app.contains("outlook")
                || app.contains("teams")
                || app.contains("slack")
                || app.contains("discord")
                || app.contains("chrome")
                || app.contains("msedge");

            if !app_is_message_surface {
                return Vec::new();
            }

            let preview = context.text_content.unwrap_or_default();
            if context.window_title.trim().is_empty() && preview.trim().is_empty() {
                return Vec::new();
            }

            vec![RawNotification {
                app: context.app_name,
                title: context.window_title,
                preview,
                sender: None,
                cc_only: false,
            }]
        }

        fn window_text(hwnd: HWND) -> String {
            let mut buffer = [0u16; 1024];
            unsafe {
                let copied = GetWindowTextW(hwnd, &mut buffer);
                if copied <= 0 {
                    return String::new();
                }
                String::from_utf16_lossy(&buffer[..copied as usize])
            }
        }

        fn window_class(hwnd: HWND) -> String {
            let mut buffer = [0u16; 256];
            unsafe {
                let copied = GetClassNameW(hwnd, &mut buffer);
                if copied <= 0 {
                    return String::new();
                }
                String::from_utf16_lossy(&buffer[..copied as usize])
            }
        }

        fn process_name_for_window(hwnd: HWND) -> Option<String> {
            let mut pid = 0u32;
            unsafe {
                let _thread_id = GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid == 0 {
                    return None;
                }

                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
                let mut buffer = [0u16; 1024];
                let mut size = buffer.len() as u32;
                let result = QueryFullProcessImageNameW(
                    process,
                    PROCESS_NAME_WIN32,
                    PWSTR(buffer.as_mut_ptr()),
                    &mut size,
                );
                let _ = CloseHandle(process);
                if result.is_err() || size == 0 {
                    return None;
                }
                let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
                let file_name = Path::new(&full_path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(full_path.as_str())
                    .to_string();
                Some(file_name)
            }
        }

        fn extract_text_from_focused_element() -> Option<String> {
            let automation: IUIAutomation =
                unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()? };
            let element = unsafe { automation.GetFocusedElement().ok()? };

            if let Some(text) = extract_text_via_text_pattern(&element) {
                return Some(text);
            }
            extract_text_via_value_pattern(&element)
        }

        fn extract_text_via_text_pattern(
            element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
        ) -> Option<String> {
            unsafe {
                let pattern: IUIAutomationTextPattern =
                    element.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
                let range = pattern.DocumentRange().ok()?;
                let text = range.GetText(4096).ok()?;
                normalize_text(text.to_string())
            }
        }

        fn extract_text_via_value_pattern(
            element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
        ) -> Option<String> {
            unsafe {
                let pattern: IUIAutomationValuePattern =
                    element.GetCurrentPatternAs(UIA_ValuePatternId).ok()?;
                let value = pattern.CurrentValue().ok()?;
                normalize_text(value.to_string())
            }
        }

        fn normalize_text(text: String) -> Option<String> {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(trimmed.chars().take(4096).collect())
        }

        fn fallback_text(window_title: &str) -> Option<String> {
            normalize_text(window_title.to_string())
        }
    }

    #[cfg(not(windows))]
    mod windows_impl {
        use crate::DesktopMonitorResult;
        use crate::{RawNotification, WindowContext};

        pub struct ComGuard;
        pub struct FocusHookGuard;

        impl ComGuard {
            pub fn initialize_for_monitor_thread() -> DesktopMonitorResult<Self> {
                Ok(Self)
            }
        }

        pub fn ensure_automation_available() -> DesktopMonitorResult<()> {
            Ok(())
        }

        pub fn install_focus_change_hook() -> Option<FocusHookGuard> {
            None
        }

        pub fn focus_event_epoch() -> u64 {
            0
        }

        pub fn capture_window_context() -> Option<WindowContext> {
            None
        }

        pub fn poll_notifications() -> Vec<RawNotification> {
            Vec::new()
        }
    }

    pub use windows_impl::{
        ComGuard, capture_window_context, ensure_automation_available, focus_event_epoch,
        install_focus_change_hook, poll_notifications,
    };
}

pub use platform::capture_window_context;

#[cfg(test)]
mod tests {
    use super::{
        ContextType, DesktopMonitor, ImportanceScorer, NotificationDetector, NotificationState,
        RawNotification, RedactionPolicy, UserPatterns, WindowContext, redact_text,
    };
    use std::time::Duration;

    #[test]
    fn context_detection_maps_known_processes() {
        assert_eq!(
            DesktopMonitor::detect_context_type("OUTLOOK.EXE", ""),
            ContextType::Email
        );
        assert_eq!(
            DesktopMonitor::detect_context_type("powerpnt.exe", ""),
            ContextType::Presentation
        );
        assert_eq!(
            DesktopMonitor::detect_context_type("code.exe", ""),
            ContextType::Code
        );
    }

    #[test]
    fn context_detection_uses_window_class_fallbacks() {
        assert_eq!(
            DesktopMonitor::detect_context_type("unknown.exe", "Chrome_WidgetWin_1"),
            ContextType::Browser
        );
    }

    #[test]
    fn window_context_is_cloneable() {
        let context = WindowContext {
            app_name: "OUTLOOK.EXE".to_string(),
            window_title: "RE: Project Update".to_string(),
            context_type: ContextType::Email,
            text_content: Some("body".to_string()),
            cursor_position: (10, 20),
        };
        let cloned = context.clone();
        assert_eq!(cloned.window_title, "RE: Project Update");
    }

    #[test]
    fn importance_scorer_applies_expected_weights() {
        let scorer = ImportanceScorer::new(UserPatterns {
            vip_senders: vec!["boss@company.com".to_string()],
            urgent_keywords: vec!["urgent".to_string()],
        });
        let score = scorer.score(&RawNotification {
            app: "Outlook".to_string(),
            title: "URGENT: Re: budget".to_string(),
            preview: "Please reply by EOD".to_string(),
            sender: Some("boss@company.com".to_string()),
            cc_only: false,
        });
        assert!(score >= 90.0);
    }

    #[tokio::test]
    async fn notification_detector_emits_ingested_events() {
        let scorer = ImportanceScorer::new(UserPatterns {
            vip_senders: vec![],
            urgent_keywords: vec!["urgent".to_string()],
        });
        let detector = NotificationDetector::new(scorer, Duration::from_millis(20))
            .with_stale_after(Duration::from_secs(2));
        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "Need response today".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });

        let mut rx = detector.monitor_notifications().await;
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("detector timeout")
            .expect("event");
        assert_eq!(event.app, "Outlook");
        assert!(event.importance > 0.0);
        assert_eq!(event.state, NotificationState::New);
    }

    #[tokio::test]
    async fn notification_detector_marks_updates() {
        let scorer = ImportanceScorer::new(UserPatterns {
            vip_senders: vec![],
            urgent_keywords: vec!["urgent".to_string()],
        });
        let detector = NotificationDetector::new(scorer, Duration::from_millis(20))
            .with_throttle_window(Duration::from_millis(1))
            .with_stale_after(Duration::from_secs(2));
        let mut rx = detector.monitor_notifications().await;

        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "Need response today".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });

        let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("first event timeout")
            .expect("first event");
        assert_eq!(first.state, NotificationState::New);

        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "Need response now".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });

        let second = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("second event timeout")
            .expect("second event");
        assert_eq!(second.state, NotificationState::Updated);
    }

    #[tokio::test]
    async fn notification_detector_throttles_rapid_updates() {
        let scorer = ImportanceScorer::new(UserPatterns {
            vip_senders: vec![],
            urgent_keywords: vec!["urgent".to_string()],
        });
        let detector = NotificationDetector::new(scorer, Duration::from_millis(20))
            .with_throttle_window(Duration::from_millis(200))
            .with_stale_after(Duration::from_secs(5));
        let mut rx = detector.monitor_notifications().await;

        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "v1".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });
        let _ = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("new event timeout");

        tokio::time::sleep(Duration::from_millis(220)).await;

        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "v2".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });
        let _updated = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("first update timeout")
            .expect("first update");

        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "v3".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });

        let throttled = tokio::time::timeout(Duration::from_millis(150), rx.recv()).await;
        assert!(
            throttled.is_err(),
            "rapid updates should be throttled and not emit immediately"
        );
    }

    #[tokio::test]
    async fn notification_detector_emits_dismissed_state() {
        let scorer = ImportanceScorer::new(UserPatterns {
            vip_senders: vec![],
            urgent_keywords: vec!["urgent".to_string()],
        });
        let detector = NotificationDetector::new(scorer, Duration::from_millis(20))
            .with_stale_after(Duration::from_millis(80));
        let mut rx = detector.monitor_notifications().await;

        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: budget".to_string(),
            preview: "Need response today".to_string(),
            sender: Some("pm@company.com".to_string()),
            cc_only: false,
        });
        let first = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("first event timeout")
            .expect("first event");
        assert_eq!(first.state, NotificationState::New);

        let dismissed = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("dismissed timeout")
            .expect("dismissed event");
        assert_eq!(dismissed.state, NotificationState::Dismissed);
    }

    #[test]
    fn redaction_masks_common_sensitive_patterns() {
        let policy = RedactionPolicy::default();
        let redacted = redact_text(
            "Contact alice@example.com with token sk-test_1234567890ABCDEFGHIJK and card 4111 1111 1111 1111",
            &policy,
        );
        assert!(!redacted.contains("alice@example.com"));
        assert!(!redacted.contains("sk-test_1234567890ABCDEFGHIJK"));
        assert!(!redacted.contains("4111 1111 1111 1111"));
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("[REDACTED_TOKEN]"));
        assert!(redacted.contains("[REDACTED_NUMBER]"));
    }

    #[tokio::test]
    async fn detector_emits_redacted_notification_content() {
        let scorer = ImportanceScorer::new(UserPatterns {
            vip_senders: vec!["alice".to_string()],
            urgent_keywords: vec!["urgent".to_string()],
        });
        let detector = NotificationDetector::new(scorer, Duration::from_millis(20))
            .with_stale_after(Duration::from_secs(2))
            .with_redaction_policy(RedactionPolicy::default());
        detector.ingest_notification(RawNotification {
            app: "Outlook".to_string(),
            title: "urgent: wire".to_string(),
            preview: "Send to alice@example.com token=ghp_1234567890ABCDEFGHIJKL".to_string(),
            sender: Some("alice@example.com".to_string()),
            cc_only: false,
        });

        let mut rx = detector.monitor_notifications().await;
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("detector timeout")
            .expect("event");
        assert!(event.preview.contains("[REDACTED_EMAIL]"));
        assert!(event.preview.contains("[REDACTED_TOKEN]"));
    }
}
pub mod file_ops;
