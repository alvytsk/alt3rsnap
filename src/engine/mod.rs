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
        // Implementation grows task-by-task in Phase 2.
        let _ = event;
        Vec::new()
    }
}
