use crate::{ScreenPt, ScreenVec};

/// A GUI-neutral input event, in screen points relative to the canvas viewport's top-left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// Move the content by `by` (positive: content moves right / down), as a wheel or
    /// two-finger scroll does.
    Pan { by: ScreenVec },
    /// Multiply the zoom by `factor` (> 1 zooms in), keeping the world point under `about`
    /// fixed on screen.
    Zoom { about: ScreenPt, factor: f32 },
}
