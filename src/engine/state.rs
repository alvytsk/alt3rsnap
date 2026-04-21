//! Finite-state machine types: State, Event, Action, Engine.

use crate::engine::geometry::{Point, Rect, ResizeAnchor};
use crate::engine::modifiers::Modifiers;

/// Opaque wrapper over the adapter's window handle. The engine is handle-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualKey {
    Alt,
    Ctrl,
    Shift,
    Win,
    Space,
    Other(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Disabled,
    Idle,
    Armed,
    Moving {
        hwnd: WindowId,
        initial_rect: Rect,
        grab: Point,
        pending_passthrough: bool,
    },
    Resizing {
        hwnd: WindowId,
        initial_rect: Rect,
        grab: Point,
        anchor: ResizeAnchor,
        pending_passthrough: bool,
    },
    PassThrough,
}

/// The target window info the adapter attaches to LeftDown/RightDown events.
#[derive(Debug, Clone)]
pub struct DragTarget {
    pub hwnd: WindowId,
    pub initial_rect: Rect,
    pub is_maximized: bool,
    pub exclude: bool, // precomputed by adapter from rule engine
}

#[derive(Debug, Clone)]
pub enum Event {
    KeyChange {
        vk: VirtualKey,
        down: bool,
    },
    LeftDown {
        cursor: Point,
        target: Option<DragTarget>,
    },
    LeftUp,
    RightDown {
        cursor: Point,
        target: Option<DragTarget>,
    },
    RightUp,
    MiddleDown {
        cursor: Point,
        target: Option<DragTarget>,
    },
    MouseMove {
        cursor: Point,
    },
    FullscreenFocused,
    FullscreenUnfocused,
    ToggleEnable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragMode {
    Move,
    Resize { anchor: ResizeAnchor },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    BeginDrag {
        hwnd: WindowId,
        initial_rect: Rect,
        grab: Point,
        mode: DragMode,
    },
    UpdateDrag {
        hwnd: WindowId,
        new_rect: Rect,
    },
    EndDrag {
        hwnd: WindowId,
    },
    RestoreIfMaximized {
        hwnd: WindowId,
        cursor: Point,
    },
    RaiseWindow {
        hwnd: WindowId,
    },
    CancelMenuActivation,
    SwallowEvent,
    UpdateTrayIcon {
        enabled: bool,
    },
    ToggleMaximize {
        hwnd: WindowId,
    },
}

/// Current modifier state snapshot; updated inline by the engine.
pub fn vk_bit(vk: VirtualKey) -> Modifiers {
    match vk {
        VirtualKey::Alt => Modifiers::ALT,
        VirtualKey::Ctrl => Modifiers::CTRL,
        VirtualKey::Shift => Modifiers::SHIFT,
        VirtualKey::Win => Modifiers::WIN,
        VirtualKey::Space => Modifiers::SPACE,
        VirtualKey::Other(_) => Modifiers::NONE,
    }
}
