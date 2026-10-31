//! `pen_render` paint → `rune::Shape`. The only place draw-list primitives meet rune.

use pen_core::WorldPt;
use pen_render::Prim;
use rune::{LineCap, Shape};

use crate::palette;

/// Appends `prims` to `shape`, in order. Coordinates pass through unchanged: world points in
/// the world layer, element points in an icon.
pub(crate) fn shape<'a>(prims: impl IntoIterator<Item = &'a Prim>) -> Shape {
    prims
        .into_iter()
        .fold(Shape::new(), |shape, prim| match prim {
            Prim::Fill { points, color } => shape.fill(pts(points), palette::color(*color)),
            Prim::Stroke {
                points,
                closed,
                width,
                color,
            } => shape.stroke_with(
                pts(points),
                *closed,
                *width,
                LineCap::Round,
                palette::color(*color),
            ),
        })
}

fn pts(points: &[WorldPt]) -> Vec<[f32; 2]> {
    points.iter().map(|p| [p.x, p.y]).collect()
}
