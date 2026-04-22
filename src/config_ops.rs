#![cfg(target_os = "windows")]

use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    alt3rsnap::config::default_config_path()
}

pub fn open_in_editor() {
    let path = config_path();
    if !path.exists() {
        let _ = alt3rsnap::config::save_to_path(&path, &alt3rsnap::config::FileConfig::default());
    }
    let path_str = path.to_string_lossy().into_owned();
    let wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = windows::Win32::UI::Shell::ShellExecuteW(
            None,
            windows::core::w!("open"),
            windows::core::PCWSTR(wide.as_ptr()),
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
}

pub fn reload() {
    let path = config_path();
    let result = alt3rsnap::config::load_from_path(&path);
    match result {
        Ok(file_cfg) => match file_cfg.to_runtime_config() {
            Ok(runtime) => {
                crate::hook::ENGINE.with(|e| {
                    let actions = e.borrow_mut().set_config(runtime.engine);
                    crate::adapter::apply_actions(&actions);
                });
                *crate::adapter_config_handle().lock().unwrap() = runtime.adapter;
            }
            Err(e) => tracing::error!("config conversion failed, keeping previous: {e}"),
        },
        Err(e) => tracing::error!("config load failed, keeping previous: {e}"),
    }
}
