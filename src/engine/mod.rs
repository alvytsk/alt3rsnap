//! Engine: pure Rust, no `windows` crate imports.

pub mod config;
pub mod geometry;
pub mod modifiers;
pub mod policy;
pub mod rules;
pub mod state;

use crate::engine::config::EngineConfig;
use crate::engine::modifiers::Modifiers;
use crate::engine::state::{Action, Event, State};

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
            _ => {} // other events handled in later tasks
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
