//! Hidden tool window that hosts the tray icon and receives hook-forwarded events.

#![cfg(target_os = "windows")]

use std::sync::OnceLock;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassExW, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_DESTROY, WNDCLASSEXW,
};

pub const TOOL_WND_CLASS: PCWSTR = w!("Alt3rSnapToolWnd");

static TOOL_HWND: OnceLock<HwndWrap> = OnceLock::new();

#[derive(Copy, Clone)]
struct HwndWrap(pub HWND);
// Safety: HWND is `!Send`/`!Sync` in `windows`, but we only access it from the main thread.
unsafe impl Send for HwndWrap {}
unsafe impl Sync for HwndWrap {}

pub fn hwnd() -> HWND { TOOL_HWND.get().copied().map(|w| w.0).unwrap_or_default() }

pub fn init_and_run() -> windows::core::Result<()> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: TOOL_WND_CLASS,
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(windows::core::Error::from_win32());
        }

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            TOOL_WND_CLASS,
            w!("Alt3rSnap"),
            WINDOW_STYLE::default(),
            0, 0, 0, 0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        )?;
        let _ = TOOL_HWND.set(HwndWrap(hwnd));

        let mut msg = MSG::default();
        loop {
            let got = GetMessageW(&mut msg, HWND::default(), 0, 0);
            if got.0 <= 0 { break; }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
