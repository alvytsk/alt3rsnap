#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    // Real Windows entry lands in Phase 5.
    eprintln!("alt3rsnap main() — not yet wired (see Phase 5 of the plan)");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("alt3rsnap is Windows-only; build with `cargo xwin build --target x86_64-pc-windows-msvc`.");
    std::process::exit(1);
}
