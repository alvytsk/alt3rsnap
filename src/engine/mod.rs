//! Engine: pure Rust, no `windows` crate imports.

pub mod config;
pub mod geometry;
pub mod modifiers;
pub mod policy;
pub mod rules;
pub mod state;

use crate::engine::config::EngineConfig;
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
            _ => {}
        }

        actions
    }

    fn reconcile_arm_state(&mut self, actions: &mut Vec<Action>) {
        // Only Idle <-> Armed react to modifier changes; drag states ignore.
        let arm_matches = self.config.policy.arm.matches(self.mods);
        self.state = match (&self.state, arm_matches) {
            (State::Idle, true) => {
                let _ = actions;  // no action for now; tray icon update in later task
                State::Armed
            }
            (State::Armed, false) => State::Idle,
            _ => std::mem::replace(&mut self.state, State::Idle),
        };
    }
}
