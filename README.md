# Alt3rSnap

A modern Rust-based Windows utility to move and resize windows by dragging with a modifier key.

Hold **Alt** and drag any window from anywhere inside it:
- **Alt + Left-drag** — move.
- **Alt + Right-drag** — resize. The cursor's sector in a 3×3 grid picks the anchor. Center resizes symmetrically.
- **Alt + Ctrl + Drag** — move or resize and raise the window to front.

Modeled after [AltSnap](https://github.com/RamonUnch/AltSnap), rewritten from scratch in Rust with a testable pure-Rust core.

## Install

1. Download `alt3rsnap.exe` from the latest [release](https://github.com/avymiatnin/alt3rsnap/releases).
2. Put it somewhere permanent, e.g. `%LOCALAPPDATA%\Alt3rSnap\`.
3. Run it. Right-click the tray icon for options.

Runs unelevated by default. For dragging admin-elevated windows (Task Manager, regedit, elevated terminals), use tray → **Restart elevated**.

## Build from source

```bash
# From WSL / Linux — cross-compile
cargo install cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc
# produces target/x86_64-pc-windows-msvc/release/alt3rsnap.exe

# From Windows — native build
cargo build --release
```

## Configuration

`%APPDATA%\Alt3rSnap\config.toml`:

```toml
[activation]
modifier = "alt"              # "alt" | "ctrl" | "shift" | "win" | combos

[behavior]
enable_move = true
enable_resize = true
raise_on_drag = false         # AltSnap default
restore_maximized_on_move = true

[resize]
center_mode = "symmetric"
center_fraction = 0.333

[exclude]
processes = []                # e.g. ["mstsc.exe", "vmware-vmx.exe"]
```

Tray menu → "Reload config" applies changes without restarting.

## Status

v0.1 MVP. Roadmap in [`docs/superpowers/specs/2026-04-20-alt3rsnap-design.md`](docs/superpowers/specs/2026-04-20-alt3rsnap-design.md) (§8).

## License

MIT OR Apache-2.0.
