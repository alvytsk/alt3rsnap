//! Engine-visible config view.

use crate::engine::policy::ActivationPolicy;
use crate::engine::rules::WindowRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddleClickAction {
    None,
    ToggleMaximize,
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
    pub middle_click_action: MiddleClickAction,
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
            middle_click_action: MiddleClickAction::None,
        }
    }
}
