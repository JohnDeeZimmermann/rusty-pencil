use crate::{ScreenPt, ScreenVec};

/// A GUI-neutral input event. Points are screen points relative to the canvas viewport's
/// top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// Move the content by `by` (positive: content moves right / down), as a wheel or
    /// two-finger scroll does.
    Pan { by: ScreenVec },
    /// Multiply the zoom by `factor` (> 1 zooms in), keeping the world point under `about`
    /// fixed on screen.
    Zoom { about: ScreenPt, factor: f32 },
    /// The primary button went down on the canvas.
    PointerDown { at: ScreenPt, mods: Modifiers },
    /// The pointer moved while the primary button is held.
    PointerMove { at: ScreenPt, mods: Modifiers },
    /// The primary button came up.
    PointerUp { at: ScreenPt, mods: Modifiers },
    /// Modifier keys changed with no pointer movement (Shift pressed mid-drag).
    Modifiers(Modifiers),
    /// The gesture was abandoned (Escape, focus lost). Nothing it started is kept.
    Cancel,
    /// A key press that no text field took.
    Key { key: Key, mods: Modifiers },
    /// Pick a tool directly (a toolbar button).
    SelectTool(crate::Tool),
}

/// Modifier keys held during an event. `ctrl` means Cmd on macOS and Ctrl elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl Modifiers {
    pub const NONE: Modifiers = Modifiers {
        shift: false,
        alt: false,
        ctrl: false,
    };
}

/// The keys the editor binds. Anything else never reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A printable character, lowercased.
    Char(char),
    Escape,
}
