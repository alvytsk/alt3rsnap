//! Engine: pure Rust, no `windows` crate imports.

pub mod config;
pub mod geometry;
pub mod modifiers;
pub mod policy;
pub mod rules;
pub mod state;

use crate::engine::config::EngineConfig;
use crate::engine::geometry::ResizeAnchor;
use crate::engine::modifiers::Modifiers;
use crate::engine::state::{Action, DragMode, Event, State};

pub struct Engine {
    state: State,
    mods: Modifiers,
    config: EngineConfig,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Engine {
            state: if config.enabled { State::Idle } else { State::Disabled },
            mods: Modifiers::NONE,
            config,
        }
    }

    pub fn state(&self) -> &State { &self.state }
    pub fn mods(&self) -> Modifiers { self.mods }
    pub fn config(&self) -> &EngineConfig { &self.config }

    /// Process one event; return actions for the adapter to execute.
    pub fn handle(&mut self, event: Event) -> Vec<Action> {
        let mut actions = Vec::with_capacity(4);

        if matches!(self.state, State::Disabled | State::PassThrough) {
            // Still update modifier state so we're correct on resume, but ignore otherwise.
            if let Event::KeyChange { vk, down } = &event {
                let bit = crate::engine::state::vk_bit(*vk);
                if !bit.is_empty() {
                    self.mods = if *down { self.mods.with(bit) } else { self.mods.without(bit) };
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
                    self.mods = if *down { self.mods.with(bit) } else { self.mods.without(bit) };
                }
                self.reconcile_arm_state(&mut actions);
            }
            Event::LeftDown { cursor, target } => {
                if let State::Armed = self.state {
                    if !self.config.enable_move { return actions; }
                    let Some(target) = target.clone() else { return actions; };
                    if target.exclude { return actions; }

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
                    });
                    actions.push(Action::SwallowEvent);

                    self.state = State::Moving {
                        hwnd: target.hwnd,
                        initial_rect: target.initial_rect,
                        grab: *cursor,
                        pending_passthrough: false,
                    };
                }
            }
            Event::MouseMove { cursor } => {
                if let State::Moving { hwnd, initial_rect, grab, .. } = &self.state {
                    let delta = cursor.delta(*grab);
                    actions.push(Action::UpdateDrag {
                        hwnd: *hwnd,
                        new_rect: initial_rect.translate_by(delta),
                    });
                } else if let State::Resizing { hwnd, initial_rect, grab, anchor, .. } = &self.state {
                    let delta = cursor.delta(*grab);
                    let new_rect = crate::engine::geometry::apply_resize(*initial_rect, *anchor, delta);
                    actions.push(Action::UpdateDrag { hwnd: *hwnd, new_rect });
                }
            }
            Event::LeftUp => {
                if let State::Moving { hwnd, pending_passthrough, .. } = &self.state {
                    let hwnd = *hwnd;
                    let pp = *pending_passthrough;
                    actions.push(Action::EndDrag { hwnd });
                    actions.push(Action::CancelMenuActivation);
                    self.state = if pp { State::PassThrough } else { State::Idle };
                    self.reconcile_arm_state(&mut actions);
                }
            }
            Event::RightDown { cursor, target } => {
                if let State::Armed = self.state {
                    if !self.config.enable_resize { return actions; }
                    let Some(target) = target.clone() else { return actions; };
                    if target.exclude { return actions; }

                    let sector = crate::engine::geometry::pick_sector(
                        target.initial_rect, *cursor, self.config.center_fraction,
                    );
                    let anchor = sector_to_anchor(sector);

                    if self.config.raise_on_drag || self.config.policy.raise.matches(self.mods) {
                        actions.push(Action::RaiseWindow { hwnd: target.hwnd });
                    }

                    actions.push(Action::BeginDrag {
                        hwnd: target.hwnd,
                        initial_rect: target.initial_rect,
                        grab: *cursor,
                        mode: DragMode::Resize { anchor },
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
                if let State::Resizing { hwnd, pending_passthrough, .. } = &self.state {
                    let hwnd = *hwnd;
                    let pp = *pending_passthrough;
                    actions.push(Action::EndDrag { hwnd });
                    actions.push(Action::CancelMenuActivation);
                    self.state = if pp { State::PassThrough } else { State::Idle };
                    self.reconcile_arm_state(&mut actions);
                }
            }
            Event::ToggleEnable => {
                match std::mem::replace(&mut self.state, State::Idle) {
                    State::Disabled => {
                        self.state = State::Idle;
                        self.reconcile_arm_state(&mut actions);
                        actions.push(Action::UpdateTrayIcon { enabled: true });
                    }
                    State::Moving { hwnd, .. } | State::Resizing { hwnd, .. } => {
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
            Event::FullscreenFocused => {
                match &mut self.state {
                    State::Idle | State::Armed => { self.state = State::PassThrough; }
                    State::Moving { pending_passthrough, .. }
                    | State::Resizing { pending_passthrough, .. } => {
                        *pending_passthrough = true;
                    }
                    _ => {}
                }
            }
            Event::FullscreenUnfocused => {
                if let State::PassThrough = self.state {
                    self.state = State::Idle;
                    self.reconcile_arm_state(&mut actions);
                } else if let State::Moving { pending_passthrough, .. }
                    | State::Resizing { pending_passthrough, .. } = &mut self.state {
                    *pending_passthrough = false;
                }
            }
        }

        actions
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
}

fn sector_to_anchor(s: crate::engine::geometry::Sector) -> ResizeAnchor {
    use crate::engine::geometry::Sector::*;
    match s {
        TopLeft     => ResizeAnchor::TopLeft,
        Top         => ResizeAnchor::Top,
        TopRight    => ResizeAnchor::TopRight,
        Left        => ResizeAnchor::Left,
        Center      => ResizeAnchor::CenterSymmetric,
        Right       => ResizeAnchor::Right,
        BottomLeft  => ResizeAnchor::BottomLeft,
        Bottom      => ResizeAnchor::Bottom,
        BottomRight => ResizeAnchor::BottomRight,
    }
}
