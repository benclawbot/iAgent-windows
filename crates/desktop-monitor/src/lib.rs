use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result as AnyhowResult;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

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
    pub app: String,
    pub title: String,
    pub preview: String,
    pub importance: f32,
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
            vec!["urgent".to_string(), "important".to_string(), "deadline".to_string()]
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
}

impl NotificationDetector {
    pub fn new(scorer: ImportanceScorer, check_interval: Duration) -> Self {
        Self {
            scorer,
            check_interval,
        }
    }

    pub async fn monitor_notifications(&self) -> mpsc::UnboundedReceiver<ImportantNotification> {
        let (tx, rx) = mpsc::unbounded_channel();
        let scorer = self.scorer.clone();
        let interval = self.check_interval;
        tokio::spawn(async move {
            loop {
                // Phase 1 baseline: source integration arrives in next iteration.
                // The detector stays idle but keeps the event channel active.
                let _ = (&tx, &scorer);
                tokio::time::sleep(interval).await;
            }
        });
        rx
    }
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
    use super::WindowContext;

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
            EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
            WINEVENT_SKIPOWNPROCESS,
            GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowTextW,
            GetWindowThreadProcessId,
        };
        use windows::core::PWSTR;

        use super::WindowContext;
        use crate::DesktopMonitorResult;

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
                    if let Some(hook) = self.foreground_hook.take() {
                        if !hook.is_invalid() {
                            let _ = UnhookWinEvent(hook);
                        }
                    }
                    if let Some(hook) = self.object_focus_hook.take() {
                        if !hook.is_invalid() {
                            let _ = UnhookWinEvent(hook);
                        }
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
                let app_name = process_name_for_window(hwnd).unwrap_or_else(|| "unknown".to_string());

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
        use super::WindowContext;
        use crate::DesktopMonitorResult;

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
    }

    pub use windows_impl::{
        ComGuard, capture_window_context, ensure_automation_available, focus_event_epoch,
        install_focus_change_hook,
    };
}

#[cfg(test)]
mod tests {
    use super::{
        ContextType, DesktopMonitor, ImportanceScorer, RawNotification, UserPatterns, WindowContext,
    };

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
}
