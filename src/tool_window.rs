//! Hidden tool window that hosts the tray icon and receives hook-forwarded events.

#![cfg(target_os = "windows")]

use std::sync::OnceLock;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassExW, SetTimer, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_COMMAND, WM_DESTROY, WM_TIMER, WNDCLASSEXW,
};

pub const TOOL_WND_CLASS: PCWSTR = w!("Alt3rSnapToolWnd");

const TIMER_ID_SWALLOW_LATCH: usize = 0x4153_0001; // "AS0001" — avoids collision with tray ids.

static TOOL_HWND: OnceLock<HwndWrap> = OnceLock::new();

#[derive(Copy, Clone)]
struct HwndWrap(pub HWND);
unsafe impl Send for HwndWrap {}
unsafe impl Sync for HwndWrap {}

pub fn hwnd() -> HWND {
    TOOL_HWND.get().copied().map(|w| w.0).unwrap_or_default()
}

pub fn create() -> windows::core::Result<HWND> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: TOOL_WND_CLASS,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            return Err(windows::core::Error::from_win32());
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            TOOL_WND_CLASS,
            w!("Alt3rSnap"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        )?;
        let _ = TOOL_HWND.set(HwndWrap(hwnd));
        let _ = SetTimer(hwnd, TIMER_ID_SWALLOW_LATCH, 250, None);
        Ok(hwnd)
    }
}

pub fn run_pump() {
    unsafe {
        let mut msg = MSG::default();
        loop {
            let got = GetMessageW(&mut msg, HWND::default(), 0, 0);
            if got.0 <= 0 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        m if m == crate::tray::WM_TRAY_CALLBACK => {
            crate::tray::on_tray_message(hwnd, wparam, lparam);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 as u32) & 0xFFFF;
            crate::tray::on_command(id);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_ID_SWALLOW_LATCH {
                let now = windows::Win32::System::SystemInformation::GetTickCount64();
                crate::adapter::swallow_latch().on_timer(now);
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
