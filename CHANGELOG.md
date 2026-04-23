# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.2.1] — 2026-04-23
### Added
- `rustfmt.toml` with explicit formatting rules
- `[lints]` table in `Cargo.toml` for manifest-level lint control
- `.github/workflows/release.yml`: tag-triggered release builds UPX-packed `.exe` + SHA-256

### Changed
- Drop unnecessary path qualifications and clean up unused imports in Windows adapter modules (surfaced by the new `unused_qualifications` lint)

### Removed
- Unused `windows` crate features: `Win32_System_Com`, `Win32_Storage_FileSystem`

## [0.2.0] — 2026-04-23
### Added
- Edge snap with translucent ghost overlay preview (proximity-based, per-zone toggles in `[snap]` config)
- Middle-click maximize/restore (`Alt` + middle-click)
- Full `[[rules]]` TOML schema: `match_process` / `match_class` / `match_title` with `exact` / `glob` / `regex`
- `center_mode = "bottom_right"` — resize anchors at top-left (AltSnap-compatible)
- `center_mode = "move"` — center sector of right-drag moves instead of resizes

## [0.1.1] — 2026-04-21
### Fixed
- Embedded icon now appears correctly in Explorer and the tray
- Autostart registry entry fixed

## [0.1.0] — 2026-04-20
### Added
- Initial release: Alt+left-drag to move, Alt+right-drag to resize (9-sector)
- Configurable modifier keys and exclusion rules via TOML
- Tray icon with enable/disable, reload config, restart elevated, autostart
- Fullscreen protection (disables engagement when a fullscreen app is focused)
- DPI-aware (Per-Monitor V2)
- Tracing log to `%APPDATA%\Alt3rSnap\logs\`
