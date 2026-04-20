//! System tray icon and context menu.

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Shell::{
    NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, SetForegroundWindow,
    TrackPopupMenu, IDI_APPLICATION, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    TPM_RIGHTBUTTON,
};

use crate::hook::ENGINE;
use alt3rsnap::engine::state::{Event as EngineEvent, State};

pub const WM_TRAY_CALLBACK: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

const ID_TOGGLE_ENABLED: u32 = 100;
const ID_OPEN_CONFIG: u32    = 101;
const ID_RELOAD_CONFIG: u32  = 102;
const ID_AUTOSTART_NORMAL: u32 = 103;
const ID_AUTOSTART_ELEVATED: u32 = 104;
const ID_RESTART_ELEVATED: u32 = 105;
const ID_RESTART_NORMAL: u32   = 106;
const ID_ABOUT: u32 = 107;
const ID_EXIT: u32  = 108;

static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn install(tool_hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: tool_hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY_CALLBACK,
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
            ..Default::default()
        };
        let tip = "Alt3rSnap";
        for (i, c) in tip.encode_utf16().enumerate() {
            nid.szTip[i] = c;
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

pub fn uninstall(tool_hwnd: HWND) {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: tool_hwnd,
            uID: 1,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

pub fn on_tray_message(tool_hwnd: HWND, _wparam: WPARAM, lparam: LPARAM) {
    let event = (lparam.0 as u32) & 0xFFFF;
    const WM_RBUTTONUP: u32 = 0x0205;
    const WM_LBUTTONUP: u32 = 0x0202;
    match event {
        WM_LBUTTONUP => toggle_enabled(),
        WM_RBUTTONUP => show_menu(tool_hwnd),
        _ => {}
    }
}

fn toggle_enabled() {
    ENGINE.with(|e| {
        let actions = e.borrow_mut().handle(EngineEvent::ToggleEnable);
        crate::adapter::apply_actions(&actions);
    });
    ENABLED.store(
        ENGINE.with(|e| !matches!(e.borrow().state(), State::Disabled)),
        Ordering::SeqCst,
    );
}

pub fn set_enabled_flag(enabled: bool) { ENABLED.store(enabled, Ordering::SeqCst); }

fn show_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu().unwrap();
        let enabled = ENABLED.load(Ordering::SeqCst);
        let _ = AppendMenuW(menu, if enabled { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING,
                            ID_TOGGLE_ENABLED as usize, w!("Enabled"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_AUTOSTART_NORMAL as usize, w!("Autostart on logon"));
        let _ = AppendMenuW(menu, MF_STRING, ID_AUTOSTART_ELEVATED as usize, w!("Autostart as elevated"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        if crate::elevate::is_elevated() {
            let _ = AppendMenuW(menu, MF_STRING, ID_RESTART_NORMAL as usize, w!("Restart normally"));
        } else {
            let _ = AppendMenuW(menu, MF_STRING, ID_RESTART_ELEVATED as usize, w!("Restart elevated"));
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_OPEN_CONFIG as usize, w!("Open config file"));
        let _ = AppendMenuW(menu, MF_STRING, ID_RELOAD_CONFIG as usize, w!("Reload config"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_ABOUT as usize, w!("About\u{2026}"));
        let _ = AppendMenuW(menu, MF_STRING, ID_EXIT as usize, w!("Exit"));

        let mut pt = windows::Win32::Foundation::POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
        let _ = DestroyMenu(menu);
    }
}

pub fn on_command(id: u32) {
    match id {
        ID_TOGGLE_ENABLED => toggle_enabled(),
        ID_OPEN_CONFIG     => { crate::config_ops::open_in_editor(); }
        ID_RELOAD_CONFIG   => { crate::config_ops::reload(); }
        ID_AUTOSTART_NORMAL => { crate::autostart::toggle_normal(); }
        ID_AUTOSTART_ELEVATED => { crate::autostart::toggle_elevated(); }
        ID_RESTART_ELEVATED => { crate::elevate::restart_elevated(); }
        ID_RESTART_NORMAL   => { crate::elevate::restart_normal(); }
        ID_ABOUT  => { show_about(); }
        ID_EXIT   => unsafe { windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0); },
        _ => {}
    }
}

fn show_about() {
    unsafe {
        let text = format!(
            "Alt3rSnap v{}\nIntegrity level: {}\nhttps://github.com/avymiatnin/alt3rsnap",
            env!("CARGO_PKG_VERSION"),
            if crate::elevate::is_elevated() { "elevated" } else { "normal" }
        );
        let wtext: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
            windows::core::PCWSTR(wtext.as_ptr()),
            w!("About Alt3rSnap"),
            windows::Win32::UI::WindowsAndMessaging::MB_OK,
        );
    }
}
