#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod dpi;

#[cfg(windows)]
fn main() {
    dpi::init();
    eprintln!("alt3rsnap main() — DPI set; more wiring in later tasks");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("alt3rsnap is Windows-only; build with `cargo xwin build --target x86_64-pc-windows-msvc`.");
    std::process::exit(1);
}
