//! `pen_render` paint → `rune::Shape`. The only place draw-list primitives meet rune.
use crate::palette;
use pen_core::{WorldPt, WorldRect};
use pen_render::Prim;
use rune::{LineCap, Shape};

/// Icon coordinates pass through unchanged.
pub(crate) fn shape<'a>(prims: impl IntoIterator<Item = &'a Prim>) -> Shape {
    paint_at(prims, WorldPt::ORIGIN)
}

/// Rune culls a canvas by its layout rect, even when its Shape overflows that rect. Give
/// world paint real bounds and rebase its points, so content survives an offscreen origin.
/// These bounds and coordinates change only with paint; camera updates stay live-only.
/// Release measurement (M3 script, 7.1 s / 427 frames, scale 1): frame CPU mean 0.338 ms,
/// p95 0.747 ms. At 3000% (9.6 s / 577 frames): mean 0.416 ms, p95 0.702 ms.
pub(crate) fn bounded(prims: &[Prim]) -> (Shape, WorldRect) {
    let bounds = prims
        .iter()
        .filter_map(|prim| {
            let (points, pad) = match prim {
                Prim::Fill { points, .. } => (points, 0.0),
                Prim::Stroke { points, width, .. } => (points, width / 2.0),
            };
            let bounds = points
                .iter()
                .map(|p| WorldRect::from_corners(*p, *p))
                .reduce(WorldRect::union)?;
            Some(WorldRect::from_centre(
                bounds.centre(),
                bounds.width() / 2.0 + pad,
                bounds.height() / 2.0 + pad,
            ))
        })
        .reduce(WorldRect::union)
        .unwrap_or_default();
    (paint_at(prims, bounds.min()), bounds)
}

fn paint_at<'a>(prims: impl IntoIterator<Item = &'a Prim>, origin: WorldPt) -> Shape {
    prims
        .into_iter()
        .fold(Shape::new(), |shape, prim| match prim {
            Prim::Fill { points, color } => shape.fill(pts(points, origin), palette::color(*color)),
            Prim::Stroke {
                points,
                closed,
                width,
                color,
            } => shape.stroke_with(
                pts(points, origin),
                *closed,
                *width,
                LineCap::Round,
                palette::color(*color),
            ),
        })
}

fn pts(points: &[WorldPt], origin: WorldPt) -> Vec<[f32; 2]> {
    points
        .iter()
        .map(|p| [p.x - origin.x, p.y - origin.y])
        .collect()
}
