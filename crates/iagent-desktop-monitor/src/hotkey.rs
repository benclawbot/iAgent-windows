//! Global hotkey registration for Windows.
//!
//! Usage:
//!   let mut manager = HotkeyManager::new()?;
//!   let rx = manager.register("Ctrl+Shift+Space")?;
//!   // rx is a tokio broadcast receiver; await events on it.
//!   manager.run(); // spawns the message-pump thread

use anyhow::Result;
#[cfg(target_os = "windows")]
use anyhow::bail;
use std::thread;
use tokio::sync::broadcast;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey, UnregisterHotKey,
    VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12,
    VK_SPACE,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage, WM_HOTKEY,
};

/// A fired hotkey event.
#[derive(Debug, Clone)]
pub struct HotkeyEvent {
    pub id: u32,
    pub combo: String,
}

pub struct HotkeyManager {
    sender: broadcast::Sender<HotkeyEvent>,
    registrations: Vec<(u32, String)>, // (id, combo)
    next_id: u32,
}

impl HotkeyManager {
    pub fn new() -> Result<Self> {
        let (sender, _) = broadcast::channel(64);
        Ok(Self {
            sender,
            registrations: Vec::new(),
            next_id: 1,
        })
    }

    /// Register a hotkey combo string like "Ctrl+Shift+Space".
    /// Returns a broadcast receiver that fires on each keypress.
    pub fn register(&mut self, combo: &str) -> Result<broadcast::Receiver<HotkeyEvent>> {
        let id = self.next_id;
        self.next_id += 1;
        self.registrations.push((id, combo.to_string()));
        Ok(self.sender.subscribe())
    }

    /// Spawn the Win32 message-pump thread. Call this once after all
    /// `register()` calls. Non-blocking; returns immediately.
    pub fn run(self) {
        let sender = self.sender.clone();
        let registrations: Vec<(u32, String)> = self.registrations.into_iter().collect();

        thread::Builder::new()
            .name("iagent-hotkey-pump".to_string())
            .spawn(move || {
                #[cfg(target_os = "windows")]
                Self::pump(registrations, sender);
                #[cfg(not(target_os = "windows"))]
                {
                    tracing::warn!("hotkey manager: not running on Windows, no-op");
                    let _ = (registrations, sender);
                }
            })
            .expect("failed to spawn hotkey thread");
    }

    #[cfg(target_os = "windows")]
    fn pump(registrations: Vec<(u32, String)>, sender: broadcast::Sender<HotkeyEvent>) {
        // Register all hotkeys from this thread (required by Win32).
        let mut registered: Vec<(u32, String)> = Vec::new();
        for (id, combo) in &registrations {
            match parse_combo(combo) {
                Ok((mods, vk)) => {
                    let ok = unsafe {
                        RegisterHotKey(
                            std::ptr::null_mut(),
                            *id as i32,
                            mods | MOD_NOREPEAT,
                            vk as u32,
                        )
                    };
                    if ok == 0 {
                        tracing::error!(
                            "RegisterHotKey failed for '{}': {:?}",
                            combo,
                            std::io::Error::last_os_error()
                        );
                    } else {
                        registered.push((*id, combo.clone()));
                        tracing::info!("Registered hotkey {}: '{}'", id, combo);
                    }
                }
                Err(e) => tracing::error!("Cannot parse hotkey '{}': {}", combo, e),
            }
        }

        // Message pump.
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            let ret = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
            if ret == 0 || ret == -1 {
                break;
            }
            if msg.message == WM_HOTKEY {
                let id = msg.wParam as u32;
                if let Some((_, combo)) = registered.iter().find(|(i, _)| *i == id) {
                    let _ = sender.send(HotkeyEvent {
                        id,
                        combo: combo.clone(),
                    });
                }
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Unregister on exit.
        for (id, _) in &registered {
            unsafe { UnregisterHotKey(std::ptr::null_mut(), *id as i32) };
        }
    }
}

/// Parse "Ctrl+Shift+Space" into (modifier_flags, virtual_key_code).
#[cfg(target_os = "windows")]
fn parse_combo(combo: &str) -> Result<(u32, u16)> {
    let mut mods: u32 = 0;
    let mut vk: Option<u16> = None;

    for part in combo.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "win" | "super" => mods |= MOD_WIN,
            key => {
                vk = Some(parse_vk(key)?);
            }
        }
    }

    match vk {
        Some(k) => Ok((mods, k)),
        None => bail!("no key specified in hotkey combo '{}'", combo),
    }
}

#[cfg(target_os = "windows")]
fn parse_vk(key: &str) -> Result<u16> {
    let k = match key {
        "space" => VK_SPACE,
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
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap().to_ascii_uppercase();
            if c.is_ascii_alphanumeric() {
                c as u16
            } else {
                bail!("unsupported key character '{}'", s)
            }
        }
        other => bail!("unknown key '{}'", other),
    };
    Ok(k)
}
