//! The document model: elements, ids, geometry, hit testing, commands and the file format.
//!
//! Knows nothing about windows, input devices or rendering. Every coordinate here is a
//! [`WorldPt`]: world points, y down, unbounded in every direction.
#![forbid(unsafe_code)]

/// A point on the infinite canvas, in world points (y down). At 100% zoom one world point is
/// one screen point. Never mix with screen coordinates; `pen-editor`'s camera converts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorldPt {
    pub x: f32,
    pub y: f32,
}

impl WorldPt {
    pub const ORIGIN: WorldPt = WorldPt { x: 0.0, y: 0.0 };

    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests;
