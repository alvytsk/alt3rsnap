#![cfg_attr(windows, windows_subsystem = "windows")]

mod logging;

#[cfg(windows)]
mod adapter;
#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod config_ops;
#[cfg(windows)]
mod dpi;
#[cfg(windows)]
mod elevate;
#[cfg(windows)]
mod fullscreen;
#[cfg(windows)]
mod hook;
#[cfg(windows)]
mod tool_window;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod win_api;

#[cfg(windows)]
fn main() {
    let _log_guard = logging::init();
    logging::install_panic_hook();
    tracing::info!("alt3rsnap starting v{}", env!("CARGO_PKG_VERSION"));

    dpi::init();

    // Load config and configure the engine first.
    let path = alt3rsnap::config::default_config_path();
    if let Ok(file) = alt3rsnap::config::load_from_path(&path) {
        if let Ok(engine_cfg) = file.to_engine_config() {
            hook::ENGINE.with(|e| {
                let _ = e.borrow_mut().set_config(engine_cfg);
            });
        }
    }

    let tool_hwnd = match tool_window::create() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("tool window: {e}");
            std::process::exit(1);
        }
    };

    hook::install().expect("hook install");
    fullscreen::install();
    tray::install(tool_hwnd);

    tool_window::run_pump();

    tray::uninstall(tool_hwnd);
    fullscreen::uninstall();
    hook::uninstall();
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "alt3rsnap is Windows-only; build with `cargo xwin build --target x86_64-pc-windows-msvc`."
    );
    std::process::exit(1);
}
