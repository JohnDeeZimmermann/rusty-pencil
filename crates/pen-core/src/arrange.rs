//! Persistent arrangement metadata and world-space transforms.
use crate::{Element, WorldPt, WorldRect};
use serde::{Deserialize, Serialize};

/// Unique within a document; membership is stored innermost first on each element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupId(pub u64);

/// Lexicographic fractional index. Extending a prefix always leaves room between keys.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ZKey(Vec<u16>);

impl ZKey {
    #[must_use]
    pub fn between(lower: Option<&Self>, upper: Option<&Self>) -> Self {
        let mut digits = Vec::new();
        let mut depth = 0;
        let mut upper = upper.map(|k| k.0.as_slice());
        loop {
            let lo = lower.and_then(|k| k.0.get(depth)).copied().unwrap_or(0);
            let hi = upper
                .and_then(|k| k.get(depth))
                .copied()
                .unwrap_or(u16::MAX);
            if hi > lo && hi - lo > 1 {
                digits.push(lo + (hi - lo) / 2);
                return Self(digits);
            }
            digits.push(lo);
            if lo < hi {
                upper = None;
            }
            depth += 1;
        }
    }

    pub(crate) fn valid(&self) -> bool {
        self.0.is_empty() || self.0.last().is_some_and(|d| *d != 0)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Rotate in world points around `centre`; positive radians turn clockwise (y down).
#[must_use]
pub fn rotate(point: WorldPt, centre: WorldPt, angle: f32) -> WorldPt {
    if angle == 0.0 {
        return point;
    }
    let (s, c) = angle.sin_cos();
    let (x, y) = (point.x - centre.x, point.y - centre.y);
    WorldPt::new(centre.x + c * x - s * y, centre.y + s * x + c * y)
}

impl Element {
    #[must_use]
    pub fn bounds(&self) -> WorldRect {
        let b = self.kind.bounds();
        if self.angle == 0.0 {
            return b;
        }
        if let crate::Kind::Ellipse(_) = self.kind {
            let (s, c) = self.angle.sin_cos();
            let (rx, ry) = (b.width() / 2.0, b.height() / 2.0);
            return WorldRect::from_centre(
                b.centre(),
                (rx * c).hypot(ry * s),
                (rx * s).hypot(ry * c),
            );
        }
        if let Some(path) = self.kind.path() {
            return path
                .world_points()
                .map(|p| rotate(p, b.centre(), self.angle))
                .map(|p| WorldRect::from_corners(p, p))
                .reduce(WorldRect::union)
                .unwrap_or(b);
        }
        let a = b.min();
        let z = b.max();
        let corners = if matches!(self.kind, crate::Kind::Diamond(_)) {
            let c = b.centre();
            [
                WorldPt::new(c.x, a.y),
                WorldPt::new(z.x, c.y),
                WorldPt::new(c.x, z.y),
                WorldPt::new(a.x, c.y),
            ]
        } else {
            [a, WorldPt::new(z.x, a.y), z, WorldPt::new(a.x, z.y)]
        };
        corners
            .into_iter()
            .map(|p| rotate(p, b.centre(), self.angle))
            .map(|p| WorldRect::from_corners(p, p))
            .reduce(WorldRect::union)
            .unwrap_or(b)
    }

    #[must_use]
    pub fn rotated(&self, centre: WorldPt, radians: f32) -> Self {
        let b = self.kind.bounds();
        let c = b.centre();
        let to = rotate(c, centre, radians);
        let mut out = self.with_bounds(b.translated(to.x - c.x, to.y - c.y));
        out.angle = (self.angle + radians).rem_euclid(std::f32::consts::TAU);
        out
    }
}

#[cfg(test)]
#[path = "arrange_tests.rs"]
mod tests;
