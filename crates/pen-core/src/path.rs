use crate::{WorldPt, WorldRect};

/// One sample relative to a path's origin, in world points (y down). Pressure is 0..=1;
/// line and arrow vertices use 0.5. Freedraw preserves pressure through simplification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathPoint {
    pub offset: WorldPt,
    pub pressure: f32,
}

/// Open path geometry. Samples are relative to `origin`, so moving a path preserves them.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub origin: WorldPt,
    pub points: Vec<PathPoint>,
}

impl Path {
    #[must_use]
    pub fn from_world(points: &[WorldPt]) -> Self {
        let origin = points.first().copied().unwrap_or_default();
        Self {
            origin,
            points: points
                .iter()
                .map(|p| PathPoint {
                    offset: WorldPt::new(p.x - origin.x, p.y - origin.y),
                    pressure: 0.5,
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn world_point(&self, index: usize) -> Option<WorldPt> {
        self.points
            .get(index)
            .map(|p| WorldPt::new(self.origin.x + p.offset.x, self.origin.y + p.offset.y))
    }

    pub fn world_points(&self) -> impl DoubleEndedIterator<Item = WorldPt> + '_ {
        self.points
            .iter()
            .map(|p| WorldPt::new(self.origin.x + p.offset.x, self.origin.y + p.offset.y))
    }

    #[must_use]
    pub fn bounds(&self) -> WorldRect {
        self.world_points()
            .filter(|p| p.is_finite())
            .map(|p| WorldRect::from_corners(p, p))
            .reduce(WorldRect::union)
            .unwrap_or_else(|| WorldRect::from_corners(self.origin, self.origin))
    }

    /// Open arrowhead on the last nonzero segment, in world points. Repeated end samples
    /// and paths shorter than two points have no direction and produce no head.
    #[must_use]
    pub fn arrowhead(&self, width: f32) -> Option<[WorldPt; 3]> {
        let tip = self.world_points().last()?;
        let from = self
            .world_points()
            .rev()
            .find(|p| (p.x - tip.x).hypot(p.y - tip.y) > 0.001)?;
        let (dx, dy) = (tip.x - from.x, tip.y - from.y);
        let len = dx.hypot(dy);
        let size = (width * 8.0).max(10.0).min(len * 0.5);
        let (ux, uy) = (dx / len, dy / len);
        Some([
            WorldPt::new(
                tip.x - size * ux - size * 0.5 * uy,
                tip.y - size * uy + size * 0.5 * ux,
            ),
            tip,
            WorldPt::new(
                tip.x - size * ux + size * 0.5 * uy,
                tip.y - size * uy - size * 0.5 * ux,
            ),
        ])
    }

    pub(crate) fn with_bounds(&self, bounds: WorldRect) -> Self {
        let before = self.bounds();
        let map = |p: WorldPt| {
            WorldPt::new(
                axis(
                    p.x,
                    before.min().x,
                    before.width(),
                    bounds.min().x,
                    bounds.width(),
                ),
                axis(
                    p.y,
                    before.min().y,
                    before.height(),
                    bounds.min().y,
                    bounds.height(),
                ),
            )
        };
        let origin = map(self.origin);
        Self {
            origin,
            points: self
                .world_points()
                .zip(&self.points)
                .map(|(p, sample)| {
                    let p = map(p);
                    PathPoint {
                        offset: WorldPt::new(p.x - origin.x, p.y - origin.y),
                        ..*sample
                    }
                })
                .collect(),
        }
    }
}

fn axis(p: f32, old_min: f32, old_span: f32, min: f32, span: f32) -> f32 {
    if old_span > f32::EPSILON {
        min + (p - old_min) / old_span * span
    } else {
        min + span / 2.0
    }
}
