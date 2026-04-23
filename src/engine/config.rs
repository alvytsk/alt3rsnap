//! Engine-visible config view.

use crate::engine::policy::ActivationPolicy;
use crate::engine::rules::WindowRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddleClickAction {
    None,
    ToggleMaximize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneToggles {
    pub top_maximize: bool,
    pub bottom_maximize: bool,
    pub left_half: bool,
    pub right_half: bool,
    pub top_left_quarter: bool,
    pub top_right_quarter: bool,
    pub bottom_left_quarter: bool,
    pub bottom_right_quarter: bool,
    pub left_third: bool,
    pub middle_third: bool,
    pub right_third: bool,
}

impl Default for ZoneToggles {
    fn default() -> Self {
        Self {
            top_maximize: true,
            bottom_maximize: false,
            left_half: true,
            right_half: true,
            top_left_quarter: true,
            top_right_quarter: true,
            bottom_left_quarter: true,
            bottom_right_quarter: true,
            left_third: false,
            middle_third: false,
            right_third: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapEngineConfig {
    pub enabled: bool,
    pub engage_px: u32,
    pub disengage_px: u32,
    pub zones: ZoneToggles,
}

impl Default for SnapEngineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engage_px: 24,
            disengage_px: 32,
            zones: ZoneToggles::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterMode {
    Symmetric,
    BottomRight,
    Move,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub enabled: bool,
    pub enable_move: bool,
    pub enable_resize: bool,
    pub raise_on_drag: bool,
    pub restore_maximized_on_move: bool,
    pub policy: ActivationPolicy,
    pub rules: Vec<WindowRule>,
    pub center_fraction: f32,
    pub center_mode: CenterMode,
    pub middle_click_action: MiddleClickAction,
    pub snap: SnapEngineConfig,
}

impl Default for EngineConfig {
    fn default() -> EngineConfig {
        EngineConfig {
            enabled: true,
            enable_move: true,
            enable_resize: true,
            raise_on_drag: false,
            restore_maximized_on_move: true,
            policy: ActivationPolicy::default(),
            rules: Vec::new(),
            center_fraction: 1.0 / 3.0,
            center_mode: CenterMode::Symmetric,
            middle_click_action: MiddleClickAction::None,
            snap: SnapEngineConfig::default(),
        }
    }
}
