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

    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// An axis-aligned rectangle in world points.
///
/// Invariant: `min.x <= max.x` and `min.y <= max.y`. Every constructor normalises, so a rect
/// dragged up and to the left is the same rect as one dragged down and to the right.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WorldRect {
    min: WorldPt,
    max: WorldPt,
}

impl WorldRect {
    #[must_use]
    pub fn contains(&self, p: WorldPt) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    #[must_use]
    pub fn contains_rect(&self, other: Self) -> bool {
        self.contains(other.min) && self.contains(other.max)
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self::from_corners(
            WorldPt::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            WorldPt::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        )
    }

    #[must_use]
    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self::from_corners(
            WorldPt::new(self.min.x + dx, self.min.y + dy),
            WorldPt::new(self.max.x + dx, self.max.y + dy),
        )
    }

    /// The rect spanned by two opposite corners, in any order.
    #[must_use]
    pub fn from_corners(a: WorldPt, b: WorldPt) -> Self {
        Self {
            min: WorldPt::new(a.x.min(b.x), a.y.min(b.y)),
            max: WorldPt::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// The rect centred on `centre` reaching `half_w` / `half_h` to each side. Negative halves
    /// are treated as their absolute value.
    #[must_use]
    pub fn from_centre(centre: WorldPt, half_w: f32, half_h: f32) -> Self {
        let (hw, hh) = (half_w.abs(), half_h.abs());
        Self {
            min: WorldPt::new(centre.x - hw, centre.y - hh),
            max: WorldPt::new(centre.x + hw, centre.y + hh),
        }
    }

    #[must_use]
    pub fn min(&self) -> WorldPt {
        self.min
    }

    #[must_use]
    pub fn max(&self) -> WorldPt {
        self.max
    }

    #[must_use]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    #[must_use]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    #[must_use]
    pub fn centre(&self) -> WorldPt {
        WorldPt::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    /// True when the rect has no area: one side is zero (or not finite).
    #[must_use]
    pub fn is_degenerate(&self) -> bool {
        !self.min.is_finite()
            || !self.max.is_finite()
            || !(self.width() > 0.0 && self.height() > 0.0)
    }
}
