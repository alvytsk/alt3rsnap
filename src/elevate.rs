#![cfg(target_os = "windows")]

use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const SENTINEL_ARG: &str = "--relaunched";

pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() { return false; }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut size,
        ).is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

fn exe_path() -> PathBuf { std::env::current_exe().unwrap_or_default() }

pub fn restart_elevated() {
    if is_elevated() { return; }
    let exe = exe_path();
    let exe_w: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let args_w: Vec<u16> = SENTINEL_ARG.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(exe_w.as_ptr()),
            lpParameters: PCWSTR(args_w.as_ptr()),
            nShow: SW_SHOWNORMAL.0 as i32,
            ..Default::default()
        };
        if ShellExecuteExW(&mut info).is_ok() {
            std::process::exit(0);
        }
    }
}

pub fn restart_normal() {
    if !is_elevated() { return; }
    let exe = exe_path();
    let cmd = format!("\"{}\"", exe.to_string_lossy());
    let cmd_w: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = windows::Win32::UI::Shell::ShellExecuteW(
            None,
            w!("open"),
            w!("explorer.exe"),
            PCWSTR(cmd_w.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
    std::process::exit(0);
}

pub fn was_relaunched() -> bool {
    std::env::args().any(|a| a == SENTINEL_ARG)
}
