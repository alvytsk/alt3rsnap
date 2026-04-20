//! Low-level mouse and keyboard hooks. All callbacks run on the main thread
//! (the thread that installed them).

#![cfg(target_os = "windows")]

use std::cell::RefCell;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE,
};

use alt3rsnap::engine::geometry::Point;
use alt3rsnap::engine::state::{Event, VirtualKey};
use alt3rsnap::engine::Engine;
use alt3rsnap::engine::config::EngineConfig;

thread_local! {
    pub static ENGINE: RefCell<Engine> = RefCell::new(Engine::new(EngineConfig::default()));
    static MOUSE_HOOK: RefCell<Option<HHOOK>> = const { RefCell::new(None) };
    static KEY_HOOK:   RefCell<Option<HHOOK>> = const { RefCell::new(None) };
}

pub fn install() -> windows::core::Result<()> {
    unsafe {
        let mh = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), HINSTANCE::default(), 0)?;
        MOUSE_HOOK.with(|h| *h.borrow_mut() = Some(mh));
        let kh = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_hook), HINSTANCE::default(), 0)?;
        KEY_HOOK.with(|h| *h.borrow_mut() = Some(kh));
    }
    Ok(())
}

pub fn uninstall() {
    unsafe {
        MOUSE_HOOK.with(|h| { if let Some(h) = h.borrow_mut().take() { let _ = UnhookWindowsHookEx(h); } });
        KEY_HOOK.with(|h| { if let Some(h) = h.borrow_mut().take() { let _ = UnhookWindowsHookEx(h); } });
    }
}

unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 { return CallNextHookEx(None, code, wparam, lparam); }
    let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
    let cursor = Point { x: info.pt.x, y: info.pt.y };

    let event = match wparam.0 as u32 {
        WM_MOUSEMOVE   => Some(Event::MouseMove { cursor }),
        WM_LBUTTONDOWN => {
            let target = crate::adapter::resolve_target(cursor);
            Some(Event::LeftDown { cursor, target })
        }
        WM_LBUTTONUP   => Some(Event::LeftUp),
        WM_RBUTTONDOWN => {
            let target = crate::adapter::resolve_target(cursor);
            Some(Event::RightDown { cursor, target })
        }
        WM_RBUTTONUP   => Some(Event::RightUp),
        _ => None,
    };

    let swallow = if let Some(ev) = event {
        let actions = ENGINE.with(|e| e.borrow_mut().handle(ev));
        crate::adapter::apply_actions(&actions)
    } else { false };

    if swallow { LRESULT(1) } else { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn key_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 { return CallNextHookEx(None, code, wparam, lparam); }
    let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    let up   = matches!(wparam.0 as u32, WM_KEYUP | WM_SYSKEYUP);
    if !(down || up) { return CallNextHookEx(None, code, wparam, lparam); }

    let vk = map_vk(info.vkCode as u16);

    if let Some(vk) = vk {
        let _ = ENGINE.with(|e| {
            let actions = e.borrow_mut().handle(Event::KeyChange { vk, down });
            crate::adapter::apply_actions(&actions)
        });
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn map_vk(code: u16) -> Option<VirtualKey> {
    let c = code as u32;
    if c == VK_MENU.0 as u32 || c == VK_LMENU.0 as u32 || c == VK_RMENU.0 as u32 {
        Some(VirtualKey::Alt)
    } else if c == VK_CONTROL.0 as u32 || c == VK_LCONTROL.0 as u32 || c == VK_RCONTROL.0 as u32 {
        Some(VirtualKey::Ctrl)
    } else if c == VK_SHIFT.0 as u32 || c == VK_LSHIFT.0 as u32 || c == VK_RSHIFT.0 as u32 {
        Some(VirtualKey::Shift)
    } else if c == VK_LWIN.0 as u32 || c == VK_RWIN.0 as u32 {
        Some(VirtualKey::Win)
    } else if c == VK_SPACE.0 as u32 {
        Some(VirtualKey::Space)
    } else {
        Some(VirtualKey::Other(code))
    }
}
