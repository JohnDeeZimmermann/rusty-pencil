use crate::{Arrowhead, Path, Props, WorldPt};

/// Canonical marker geometry shared by painting and hit testing, in world points.
#[derive(Debug, Clone, PartialEq)]
pub struct Head {
    pub points: Vec<WorldPt>,
    pub closed: bool,
    pub filled: bool,
}

#[must_use]
pub fn arrowheads(path: &Path, props: &Props) -> Vec<Head> {
    if !props.stroke_width.is_finite()
        || props.stroke_width <= 0.0
        || path.world_points().any(|p| !p.is_finite())
    {
        return Vec::new();
    }
    let points: Vec<_> = path.world_points().collect();
    let mut heads = Vec::new();
    for (start, head) in [(true, props.start_head), (false, props.end_head)] {
        if head == Arrowhead::None {
            continue;
        }
        let Some(tip) = (if start { points.first() } else { points.last() }).copied() else {
            continue;
        };
        let other = if start {
            points.iter().copied().find(|p| *p != tip)
        } else {
            points.iter().rev().copied().find(|p| *p != tip)
        };
        let Some(other) = other else {
            continue;
        };
        let length = (tip.x - other.x).hypot(tip.y - other.y);
        if !length.is_finite() || length < 0.001 {
            continue;
        }
        let size = (props.stroke_width * 8.0).max(10.0).min(length * 0.5);
        let (dx, dy) = ((tip.x - other.x) / length, (tip.y - other.y) / length);
        let p = |back: f32, side: f32| {
            WorldPt::new(tip.x - dx * back - dy * side, tip.y - dy * back + dx * side)
        };
        let (polygon, closed, filled) = match head {
            Arrowhead::None => continue,
            Arrowhead::Arrow => (
                vec![p(size, size * 0.5), tip, p(size, -size * 0.5)],
                false,
                false,
            ),
            Arrowhead::Bar => (vec![p(0.0, size * 0.5), p(0.0, -size * 0.5)], false, false),
            Arrowhead::Triangle | Arrowhead::TriangleOutline => (
                vec![tip, p(size, size * 0.5), p(size, -size * 0.5)],
                true,
                head == Arrowhead::Triangle,
            ),
            Arrowhead::Diamond | Arrowhead::DiamondOutline => (
                vec![
                    tip,
                    p(size * 0.5, size * 0.4),
                    p(size, 0.0),
                    p(size * 0.5, -size * 0.4),
                ],
                true,
                head == Arrowhead::Diamond,
            ),
            Arrowhead::Dot | Arrowhead::Circle | Arrowhead::CircleOutline => {
                let radius = size * 0.35;
                let n = (std::f32::consts::PI
                    / (1.0 - crate::TOLERANCE / radius.max(crate::TOLERANCE)).acos())
                .ceil()
                .clamp(16.0, 512.0) as usize;
                let poly = (0..n)
                    .map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        p(radius + radius * angle.cos(), radius * angle.sin())
                    })
                    .collect();
                (poly, true, head != Arrowhead::CircleOutline)
            }
        };
        heads.push(Head {
            points: polygon,
            closed,
            filled,
        });
    }
    heads
}
