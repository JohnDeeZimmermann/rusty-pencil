use crate::{Handle, Modifiers, ScreenPt, Selection};
use pen_core::{Element, WorldPt, WorldRect};

#[derive(Debug, Clone)]
pub(crate) enum Action {
    Move(Vec<Element>),
    Resize {
        original: Element,
        handle: Handle,
    },
    Box,
    /// Shift removes the clicked element without starting a move.
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Pressed,
    Dragging,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectionDrag {
    pub(crate) action: Action,
    pub(crate) phase: Phase,
    pub(crate) before: Selection,
    pub(crate) origin: WorldPt,
    pub(crate) current: WorldPt,
    pub(crate) start: ScreenPt,
    pub(crate) pointer: ScreenPt,
    pub(crate) mods: Modifiers,
}

impl SelectionDrag {
    pub(crate) fn marquee(&self) -> Option<WorldRect> {
        (matches!(self.action, Action::Box) && self.phase == Phase::Dragging)
            .then(|| WorldRect::from_corners(self.origin, self.current))
    }
}
