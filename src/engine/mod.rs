//! Engine: pure Rust, no `windows` crate imports.

pub mod config;
pub mod geometry;
pub mod modifiers;
pub mod policy;
pub mod rules;
pub mod snap;
pub mod state;

use crate::engine::config::{CenterMode, EngineConfig, MiddleClickAction};
use crate::engine::geometry::ResizeAnchor;
use crate::engine::modifiers::Modifiers;
use crate::engine::state::{Action, DragMode, DragOrigin, Event, State, WindowId};

/// Reason for leaving `State::Moving`. Internal to the FSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitMovingReason {
    LeftUp,
    /// Reserved for Task C5 (`Event::DragAborted` handler). Not yet constructed.
    #[allow(dead_code)]
    DragAborted,
    DisabledFromToggle,
    DisabledFromReload,
}

pub struct Engine {
    state: State,
    mods: Modifiers,
    config: EngineConfig,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Engine {
            state: if config.enabled {
                State::Idle
            } else {
                State::Disabled
            },
            mods: Modifiers::NONE,
            config,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn mods(&self) -> Modifiers {
        self.mods
    }
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Process one event; return actions for the adapter to execute.
    pub fn handle(&mut self, event: Event) -> Vec<Action> {
        let mut actions = Vec::with_capacity(4);

        if matches!(self.state, State::Disabled | State::PassThrough) {
            // Still update modifier state so we're correct on resume, but ignore otherwise.
            if let Event::KeyChange { vk, down } = &event {
                let bit = crate::engine::state::vk_bit(*vk);
                if !bit.is_empty() {
                    self.mods = if *down {
                        self.mods.with(bit)
                    } else {
                        self.mods.without(bit)
                    };
                }
            }
            // ToggleEnable and FullscreenFocused/FullscreenUnfocused still need to be processed.
            match &event {
                Event::ToggleEnable | Event::FullscreenUnfocused | Event::FullscreenFocused => {
                    // fall through to main match below
                }
                _ => return actions,
            }
        }

        match &event {
            Event::KeyChange { vk, down } => {
                let bit = crate::engine::state::vk_bit(*vk);
                if !bit.is_empty() {
                    self.mods = if *down {
                        self.mods.with(bit)
                    } else {
                        self.mods.without(bit)
                    };
                }
                self.reconcile_arm_state(&mut actions);
            }
            Event::LeftDown { cursor, target } => {
                if let State::Armed = self.state {
                    if !self.config.enable_move {
                        return actions;
                    }
                    let Some(target) = target.clone() else {
                        return actions;
                    };
                    if target.exclude {
                        return actions;
                    }

                    if target.is_maximized && self.config.restore_maximized_on_move {
                        actions.push(Action::RestoreIfMaximized {
                            hwnd: target.hwnd,
                            cursor: *cursor,
                        });
                        // After restore, the adapter will re-populate initial_rect in a
                        // follow-up LeftDown with is_maximized=false. For simplicity we
                        // still begin the drag immediately; the adapter is responsible
                        // for handing us a restored rect before BeginDrag executes.
                    }

                    if self.config.raise_on_drag || self.config.policy.raise.matches(self.mods) {
                        actions.push(Action::RaiseWindow { hwnd: target.hwnd });
                    }

                    actions.push(Action::BeginDrag {
                        hwnd: target.hwnd,
                        initial_rect: target.initial_rect,
                        grab: *cursor,
                        mode: DragMode::Move,
                        snap: None,
                    });
                    actions.push(Action::SwallowEvent);

                    self.state = State::Moving {
                        hwnd: target.hwnd,
                        initial_rect: target.initial_rect,
                        grab: *cursor,
                        drag_origin: DragOrigin::PrimaryButton,
                        pending_passthrough: false,
                        snap_session: None,
                    };
                }
            }
            Event::MiddleDown { cursor: _, target } => {
                if let State::Armed = self.state {
                    let Some(target) = target.clone() else {
                        return actions;
                    };
                    if target.exclude {
                        return actions;
                    }
                    match self.config.middle_click_action {
                        MiddleClickAction::None => {}
                        MiddleClickAction::ToggleMaximize => {
                            actions.push(Action::ToggleMaximize { hwnd: target.hwnd });
                            actions.push(Action::SwallowEvent);
                        }
                    }
                }
            }
            Event::MouseMove { cursor } => {
                if let State::Moving {
                    hwnd,
                    initial_rect,
                    grab,
                    ..
                } = &self.state
                {
                    let delta = cursor.delta(*grab);
                    actions.push(Action::UpdateDrag {
                        hwnd: *hwnd,
                        new_rect: initial_rect.translate_by(delta),
                    });
                } else if let State::Resizing {
                    hwnd,
                    initial_rect,
                    grab,
                    anchor,
                    ..
                } = &self.state
                {
                    let delta = cursor.delta(*grab);
                    let new_rect =
                        crate::engine::geometry::apply_resize(*initial_rect, *anchor, delta);
                    actions.push(Action::UpdateDrag {
                        hwnd: *hwnd,
                        new_rect,
                    });
                }
            }
            Event::LeftUp => {
                if matches!(self.state, State::Moving { .. }) {
                    if let Some((_hwnd, pp)) =
                        self.exit_moving(ExitMovingReason::LeftUp, &mut actions)
                    {
                        self.state = if pp { State::PassThrough } else { State::Idle };
                        self.reconcile_arm_state(&mut actions);
                    }
                }
            }
            Event::RightDown { cursor, target } => {
                if let State::Armed = self.state {
                    if !self.config.enable_resize {
                        return actions;
                    }
                    let Some(target) = target.clone() else {
                        return actions;
                    };
                    if target.exclude {
                        return actions;
                    }

                    let sector = crate::engine::geometry::pick_sector(
                        target.initial_rect,
                        *cursor,
                        self.config.center_fraction,
                    );

                    if self.config.raise_on_drag || self.config.policy.raise.matches(self.mods) {
                        actions.push(Action::RaiseWindow { hwnd: target.hwnd });
                    }

                    // center_mode = Move: center sector routes to Moving instead of Resizing.
                    if sector == crate::engine::geometry::Sector::Center
                        && self.config.center_mode == CenterMode::Move
                    {
                        actions.push(Action::BeginDrag {
                            hwnd: target.hwnd,
                            initial_rect: target.initial_rect,
                            grab: *cursor,
                            mode: DragMode::Move,
                            snap: None,
                        });
                        actions.push(Action::SwallowEvent);
                        self.state = State::Moving {
                            hwnd: target.hwnd,
                            initial_rect: target.initial_rect,
                            grab: *cursor,
                            drag_origin: DragOrigin::CenterMoveMode,
                            pending_passthrough: false,
                            snap_session: None,
                        };
                        return actions;
                    }

                    let anchor = sector_to_anchor(sector, self.config.center_mode);

                    actions.push(Action::BeginDrag {
                        hwnd: target.hwnd,
                        initial_rect: target.initial_rect,
                        grab: *cursor,
                        mode: DragMode::Resize { anchor },
                        snap: None,
                    });
                    actions.push(Action::SwallowEvent);

                    self.state = State::Resizing {
                        hwnd: target.hwnd,
                        initial_rect: target.initial_rect,
                        grab: *cursor,
                        anchor,
                        pending_passthrough: false,
                    };
                }
            }
            Event::RightUp => {
                let end = match &self.state {
                    State::Resizing {
                        hwnd,
                        pending_passthrough,
                        ..
                    } => Some((*hwnd, *pending_passthrough)),
                    State::Moving {
                        hwnd,
                        pending_passthrough,
                        drag_origin: DragOrigin::CenterMoveMode,
                        ..
                    } => Some((*hwnd, *pending_passthrough)),
                    _ => None,
                };
                if let Some((hwnd, pp)) = end {
                    actions.push(Action::EndDrag { hwnd });
                    actions.push(Action::CancelMenuActivation);
                    self.state = if pp { State::PassThrough } else { State::Idle };
                    self.reconcile_arm_state(&mut actions);
                }
            }
            Event::ToggleEnable => {
                if matches!(self.state, State::Moving { .. }) {
                    let _ = self.exit_moving(ExitMovingReason::DisabledFromToggle, &mut actions);
                    self.state = State::Disabled;
                    actions.push(Action::UpdateTrayIcon { enabled: false });
                } else {
                    match std::mem::replace(&mut self.state, State::Idle) {
                        State::Disabled => {
                            self.state = State::Idle;
                            self.reconcile_arm_state(&mut actions);
                            actions.push(Action::UpdateTrayIcon { enabled: true });
                        }
                        State::Resizing { hwnd, .. } => {
                            actions.push(Action::EndDrag { hwnd });
                            actions.push(Action::CancelMenuActivation);
                            self.state = State::Disabled;
                            actions.push(Action::UpdateTrayIcon { enabled: false });
                        }
                        _other => {
                            self.state = State::Disabled;
                            actions.push(Action::UpdateTrayIcon { enabled: false });
                        }
                    }
                }
            }
            Event::FullscreenFocused => match &mut self.state {
                State::Idle | State::Armed => {
                    self.state = State::PassThrough;
                }
                State::Moving {
                    pending_passthrough,
                    ..
                }
                | State::Resizing {
                    pending_passthrough,
                    ..
                } => {
                    *pending_passthrough = true;
                }
                _ => {}
            },
            Event::FullscreenUnfocused => {
                if let State::PassThrough = self.state {
                    self.state = State::Idle;
                    self.reconcile_arm_state(&mut actions);
                } else if let State::Moving {
                    pending_passthrough,
                    ..
                }
                | State::Resizing {
                    pending_passthrough,
                    ..
                } = &mut self.state
                {
                    *pending_passthrough = false;
                }
            }
            Event::DragAborted { .. } => {
                // Handler logic added in Task C5.
            }
        }

        actions
    }

    /// Central teardown path from `State::Moving`. Emits `HideSnapPreview` iff the
    /// session had an engagement, then emits reason-specific actions. Returns
    /// `(hwnd, pending_passthrough)` so the caller can set the next state.
    /// Does NOT mutate `self.state` — caller is responsible for the next state transition.
    fn exit_moving(
        &mut self,
        reason: ExitMovingReason,
        actions: &mut Vec<Action>,
    ) -> Option<(WindowId, bool)> {
        let (hwnd, had_engaged, pending_passthrough) = match &self.state {
            State::Moving {
                hwnd,
                snap_session,
                pending_passthrough,
                ..
            } => (
                *hwnd,
                snap_session.as_ref().and_then(|s| s.engaged.clone()),
                *pending_passthrough,
            ),
            _ => return None,
        };

        if had_engaged.is_some() {
            actions.push(Action::HideSnapPreview);
        }

        match reason {
            ExitMovingReason::LeftUp => {
                if let Some(ez) = had_engaged {
                    actions.push(Action::ApplySnapRect {
                        hwnd,
                        rect: ez.target_rect,
                    });
                }
                actions.push(Action::EndDrag { hwnd });
                actions.push(Action::CancelMenuActivation);
            }
            ExitMovingReason::DragAborted => {
                actions.push(Action::EndDrag { hwnd });
            }
            ExitMovingReason::DisabledFromToggle | ExitMovingReason::DisabledFromReload => {
                actions.push(Action::EndDrag { hwnd });
                actions.push(Action::CancelMenuActivation);
            }
        }

        Some((hwnd, pending_passthrough))
    }

    fn reconcile_arm_state(&mut self, actions: &mut Vec<Action>) {
        let arm_matches = self.config.policy.arm.matches(self.mods);
        self.state = match (std::mem::replace(&mut self.state, State::Idle), arm_matches) {
            (State::Idle, true) => {
                let _ = actions;
                State::Armed
            }
            (State::Armed, false) => State::Idle,
            (other, _) => other,
        };
    }

    /// Replace the current config (hot reload from tray). Preserves state except that
    /// if the new config's `enabled=false` we transition to Disabled; if toggled back
    /// on, we transition to Idle and re-evaluate modifiers.
    pub fn set_config(&mut self, cfg: EngineConfig) -> Vec<Action> {
        let mut actions = Vec::new();
        let was_enabled = !matches!(self.state, State::Disabled);
        self.config = cfg;
        if self.config.enabled && !was_enabled {
            self.state = State::Idle;
            self.reconcile_arm_state(&mut actions);
            actions.push(Action::UpdateTrayIcon { enabled: true });
        } else if !self.config.enabled && was_enabled {
            if matches!(self.state, State::Moving { .. }) {
                let _ = self.exit_moving(ExitMovingReason::DisabledFromReload, &mut actions);
            } else if let State::Resizing { hwnd, .. } = &self.state {
                let hwnd = *hwnd;
                actions.push(Action::EndDrag { hwnd });
                actions.push(Action::CancelMenuActivation);
            }
            self.state = State::Disabled;
            actions.push(Action::UpdateTrayIcon { enabled: false });
        }
        actions
    }
}

fn sector_to_anchor(s: crate::engine::geometry::Sector, center_mode: CenterMode) -> ResizeAnchor {
    use crate::engine::geometry::Sector::*;
    match s {
        TopLeft => ResizeAnchor::TopLeft,
        Top => ResizeAnchor::Top,
        TopRight => ResizeAnchor::TopRight,
        Left => ResizeAnchor::Left,
        Center => match center_mode {
            CenterMode::BottomRight => ResizeAnchor::TopLeft,
            _ => ResizeAnchor::CenterSymmetric,
        },
        Right => ResizeAnchor::Right,
        BottomLeft => ResizeAnchor::BottomLeft,
        Bottom => ResizeAnchor::Bottom,
        BottomRight => ResizeAnchor::BottomRight,
    }
}
