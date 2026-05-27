use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::personal_layer::{
    BackgroundJob, ClipboardEntry, PersonalSettings, PersonalStore, ProactiveSuggestion, Reminder,
    RuntimeTickInput, SnippetExpansion,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonConfig {
    pub tick_interval_seconds: u64,
    pub run_jobs: bool,
    pub capture_clipboard: bool,
    pub capture_active_window: bool,
    pub snippet_expansion_hook: bool,
    pub headless: bool,
}

impl Default for PersonalDaemonConfig {
    fn default() -> Self {
        Self {
            tick_interval_seconds: 15,
            run_jobs: true,
            capture_clipboard: true,
            capture_active_window: true,
            snippet_expansion_hook: true,
            headless: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonSnapshot {
    pub clipboard_content: Option<String>,
    pub active_app: Option<String>,
    pub active_window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonalDaemonTick {
    pub due_reminders: Vec<Reminder>,
    pub captured_clipboard: Option<ClipboardEntry>,
    pub completed_job: Option<BackgroundJob>,
    pub suggestions: Vec<ProactiveSuggestion>,
    pub notifications: Vec<PersonalDaemonNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonNotification {
    pub title: String,
    pub body: String,
    pub urgency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalDaemonStatus {
    pub settings: PersonalSettings,
    pub pending_reminders: usize,
    pub pending_jobs: usize,
    pub recent_clipboard_entries: usize,
    pub recent_app_windows: usize,
    pub saved_layouts: usize,
    pub timeline_entries: usize,
    pub project_workspaces: usize,
}

pub fn personal_daemon_status(store: &PersonalStore) -> Result<PersonalDaemonStatus> {
    let settings = store.settings()?;
    let pending_reminders = store.list_pending_reminders()?.len();
    let pending_jobs = store
        .list_jobs()?
        .into_iter()
        .filter(|job| job.status == "pending" || job.status == "running")
        .count();
    let recent_clipboard_entries = store
        .recent_clipboard(settings.max_clipboard_entries)?
        .len();
    let recent_app_windows = store.list_recent_app_windows(100)?.len();
    let saved_layouts = store.list_window_layouts()?.len();
    let panel = store.control_panel_summary()?;

    Ok(PersonalDaemonStatus {
        settings,
        pending_reminders,
        pending_jobs,
        recent_clipboard_entries,
        recent_app_windows,
        saved_layouts,
        timeline_entries: panel.timeline_entries,
        project_workspaces: panel.project_workspaces,
    })
}

pub fn run_personal_daemon_tick(
    store: &PersonalStore,
    snapshot: PersonalDaemonSnapshot,
    run_one_job: bool,
) -> Result<PersonalDaemonTick> {
    let tick = store.run_runtime_tick(RuntimeTickInput {
        as_of: chrono::Utc::now().to_rfc3339(),
        clipboard_content: snapshot.clipboard_content,
        active_app: snapshot.active_app,
        active_window_title: snapshot.active_window_title,
        run_one_job,
    })?;

    let mut notifications = Vec::new();
    for reminder in &tick.due_reminders {
        notifications.push(PersonalDaemonNotification {
            title: format!("Reminder: {}", reminder.title),
            body: reminder
                .note
                .clone()
                .or_else(|| reminder.source_title.clone())
                .unwrap_or_else(|| "A contextual reminder is due.".to_string()),
            urgency: "high".to_string(),
        });
    }

    if let Some(job) = &tick.completed_job {
        notifications.push(PersonalDaemonNotification {
            title: format!("Background job {}", job.status),
            body: format!("{} [{}]", job.description, job.kind),
            urgency: if job.status == "failed" {
                "high".to_string()
            } else {
                "normal".to_string()
            },
        });
    }

    for suggestion in &tick.suggestions {
        notifications.push(PersonalDaemonNotification {
            title: suggestion.title.clone(),
            body: suggestion.detail.clone().unwrap_or_default(),
            urgency: "low".to_string(),
        });
    }

    Ok(PersonalDaemonTick {
        due_reminders: tick.due_reminders,
        captured_clipboard: tick.captured_clipboard,
        completed_job: tick.completed_job,
        suggestions: tick.suggestions,
        notifications,
    })
}

pub async fn run_personal_daemon(config: PersonalDaemonConfig) -> Result<()> {
    let store = PersonalStore::load()?;
    let _snippet_hook = start_snippet_expansion_hook(&config);
    let mut interval =
        tokio::time::interval(Duration::from_secs(config.tick_interval_seconds.max(1)));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                let snapshot = capture_snapshot(&config);
                let tick = run_personal_daemon_tick(&store, snapshot, config.run_jobs)?;
                emit_notifications(&tick, config.headless);
            }
        }
    }

    Ok(())
}

pub fn apply_snippet_expansion_to_buffer(
    store: &PersonalStore,
    buffer: &mut String,
    app_name: Option<&str>,
) -> Result<Option<SnippetExpansion>> {
    let expansion = store.expand_typed_snippet(buffer, app_name)?;
    if expansion.is_some() {
        buffer.clear();
    }
    Ok(expansion)
}

struct SnippetHookGuard {
    #[cfg(target_os = "windows")]
    _thread: std::thread::JoinHandle<()>,
}

fn start_snippet_expansion_hook(config: &PersonalDaemonConfig) -> Option<SnippetHookGuard> {
    if !config.snippet_expansion_hook {
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        windows_snippet_hook::start()
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn capture_snapshot(config: &PersonalDaemonConfig) -> PersonalDaemonSnapshot {
    let mut snapshot = PersonalDaemonSnapshot::default();

    if config.capture_clipboard {
        snapshot.clipboard_content = read_clipboard_text();
    }

    if config.capture_active_window
        && let Some(context) = desktop_monitor::capture_window_context()
    {
        snapshot.active_app = Some(context.app_name);
        snapshot.active_window_title = Some(context.window_title);
    }

    snapshot
}

fn read_clipboard_text() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

fn emit_notifications(tick: &PersonalDaemonTick, headless: bool) {
    if !headless {
        // Native toast/tray rendering is expected to subscribe to the same tick data.
        // The headless path keeps the first product daemon shippable in service mode.
        return;
    }

    for notification in &tick.notifications {
        println!(
            "[personal-daemon] {} [{}]: {}",
            notification.title, notification.urgency, notification.body
        );
    }
}

#[cfg(target_os = "windows")]
mod windows_snippet_hook {
    use std::mem::size_of;
    use std::sync::{Mutex, OnceLock, mpsc};
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
        VK_BACK,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
        SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN,
        WM_SYSKEYDOWN,
    };

    use super::{
        PersonalStore, SnippetExpansion, SnippetHookGuard, apply_snippet_expansion_to_buffer,
    };

    const MAX_TYPED_BUFFER_CHARS: usize = 128;

    #[derive(Default)]
    struct SnippetHookState {
        buffer: String,
    }

    static SNIPPET_HOOK_STATE: OnceLock<Mutex<SnippetHookState>> = OnceLock::new();

    pub(super) fn start() -> Option<SnippetHookGuard> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("iagent-snippet-hook".to_string())
            .spawn(move || run_message_pump(ready_tx))
            .ok()?;

        if ready_rx.recv_timeout(Duration::from_secs(2)).ok()? {
            Some(SnippetHookGuard { _thread: thread })
        } else {
            None
        }
    }

    fn run_message_pump(ready_tx: mpsc::Sender<bool>) {
        let hook = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), std::ptr::null_mut(), 0)
        };
        if hook.is_null() {
            let _ = ready_tx.send(false);
            return;
        }
        let _ = ready_tx.send(true);

        let mut msg = MSG {
            hwnd: std::ptr::null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
        };

        while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && (wparam as u32 == WM_KEYDOWN || wparam as u32 == WM_SYSKEYDOWN) {
            let event = unsafe { *(lparam as *const KBDLLHOOKSTRUCT) };
            if event.flags & LLKHF_INJECTED == 0
                && let Some(expansion) = process_key_event(event.vkCode)
            {
                send_snippet_expansion(&expansion);
            }
        }

        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }

    fn process_key_event(vk_code: u32) -> Option<SnippetExpansion> {
        let state = SNIPPET_HOOK_STATE.get_or_init(|| Mutex::new(SnippetHookState::default()));
        let mut state = state.lock().ok()?;

        match key_char(vk_code) {
            KeyChar::Append(value) => {
                state.buffer.push(value);
                trim_buffer(&mut state.buffer);
            }
            KeyChar::Backspace => {
                state.buffer.pop();
                return None;
            }
            KeyChar::Clear => {
                state.buffer.clear();
                return None;
            }
            KeyChar::Ignore => return None,
        }

        let store = PersonalStore::load().ok()?;
        let active_app = desktop_monitor::capture_window_context().map(|context| context.app_name);
        apply_snippet_expansion_to_buffer(&store, &mut state.buffer, active_app.as_deref())
            .ok()
            .flatten()
    }

    enum KeyChar {
        Append(char),
        Backspace,
        Clear,
        Ignore,
    }

    fn key_char(vk_code: u32) -> KeyChar {
        match vk_code {
            0x08 => KeyChar::Backspace,
            0x0D | 0x1B => KeyChar::Clear,
            0x20 => KeyChar::Append(' '),
            0x30..=0x39 => char::from_u32(vk_code).map_or(KeyChar::Ignore, KeyChar::Append),
            0x41..=0x5A => char::from_u32(vk_code + 32).map_or(KeyChar::Ignore, KeyChar::Append),
            0xBD => KeyChar::Append('-'),
            0xBE => KeyChar::Append('.'),
            0xBF => KeyChar::Append('/'),
            _ => KeyChar::Ignore,
        }
    }

    fn trim_buffer(buffer: &mut String) {
        let extra_chars = buffer
            .chars()
            .count()
            .saturating_sub(MAX_TYPED_BUFFER_CHARS);
        if extra_chars > 0 {
            let byte_index = buffer
                .char_indices()
                .nth(extra_chars)
                .map(|(index, _)| index)
                .unwrap_or(buffer.len());
            buffer.drain(..byte_index);
        }
    }

    fn send_snippet_expansion(expansion: &SnippetExpansion) {
        for _ in expansion.trigger.chars() {
            send_virtual_key(VK_BACK, false);
            send_virtual_key(VK_BACK, true);
        }

        for code_unit in expansion.replacement.encode_utf16() {
            send_unicode_key(code_unit, false);
            send_unicode_key(code_unit, true);
        }
    }

    fn send_virtual_key(key: u16, key_up: bool) {
        let flags = if key_up { KEYEVENTF_KEYUP } else { 0 };
        send_keyboard_input(key, 0, flags);
    }

    fn send_unicode_key(code_unit: u16, key_up: bool) {
        let flags = KEYEVENTF_UNICODE | if key_up { KEYEVENTF_KEYUP } else { 0 };
        send_keyboard_input(0, code_unit, flags);
    }

    fn send_keyboard_input(w_vk: u16, w_scan: u16, flags: u32) {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: w_vk,
                    wScan: w_scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            let _ = SendInput(1, &input, size_of::<INPUT>() as i32);
        }
    }
}
