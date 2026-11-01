//! World-space patterns are generated once with element geometry, never with the camera.
use crate::{DrawList, Prim};
use pen_core::{FillStyle, Props, StrokeStyle, WorldPt};

pub(crate) fn finish(out: &mut DrawList, first: usize, props: &Props) {
    let mut result = Vec::new();
    for mut prim in out.prims.drain(first..) {
        let color = match &mut prim {
            Prim::Fill { color, .. } | Prim::Stroke { color, .. } | Prim::Text { color, .. } => {
                color
            }
        };
        color.a = (u16::from(color.a) * u16::from(props.opacity.min(100)) / 100) as u8;
        match prim {
            Prim::Fill { points, color } if props.fill != FillStyle::Solid => {
                hatch(&points, color, false, &mut result);
                if props.fill == FillStyle::CrossHatch {
                    hatch(&points, color, true, &mut result);
                }
            }
            Prim::Stroke {
                points,
                closed,
                width,
                color,
            } if props.stroke_style != StrokeStyle::Solid => {
                let (dash, gap) = match props.stroke_style {
                    StrokeStyle::Dashed => (8.0, 6.0),
                    StrokeStyle::Dotted => (0.01, 5.0_f32.max(width * 2.0)),
                    StrokeStyle::Solid => unreachable!(),
                };
                for segment in pattern(&points, closed, dash, gap) {
                    result.push(Prim::Stroke {
                        points: segment,
                        closed: false,
                        width,
                        color,
                    });
                }
            }
            other => result.push(other),
        }
    }
    out.prims.extend(result);
}

/// Carries phase across corners rather than restarting the dash at every flattened point.
pub(crate) fn pattern(points: &[WorldPt], closed: bool, dash: f32, gap: f32) -> Vec<Vec<WorldPt>> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut current = Vec::new();
    let mut remaining = dash;
    let mut drawing = true;
    let count = points.len() - usize::from(!closed);
    for i in 0..count {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let length = (b.x - a.x).hypot(b.y - a.y);
        if !length.is_finite() || length <= 0.0001 {
            continue;
        }
        let at = |t: f32| {
            WorldPt::new(
                a.x + (b.x - a.x) * (t / length),
                a.y + (b.y - a.y) * (t / length),
            )
        };
        let mut distance = 0.0;
        while distance < length && result.len() < 100_000 {
            let step = remaining.min(length - distance);
            if drawing {
                if current.is_empty() {
                    current.push(at(distance));
                }
                current.push(at(distance + step));
            }
            distance += step;
            remaining -= step;
            if remaining <= 0.00001 {
                if drawing && !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
                drawing = !drawing;
                remaining = if drawing { dash } else { gap };
            }
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

fn hatch(points: &[WorldPt], color: pen_core::Srgba, cross: bool, out: &mut Vec<Prim>) {
    // Rotate by 45°: clipping a horizontal scanline against the polygon is exact even at
    // rounded corners, and a half-open edge rule avoids duplicate vertex intersections.
    // Keep the hatch phase attached to the shape, so retained drag paint matches the
    // committed geometry instead of changing its pattern when the pointer is released.
    let origin = points.first().copied().unwrap_or_default();
    let sign = if cross { -1.0 } else { 1.0 };
    let k = std::f32::consts::FRAC_1_SQRT_2;
    let rotate = |p: WorldPt| {
        let (x, y) = (p.x - origin.x, p.y - origin.y);
        WorldPt::new((x + sign * y) * k, (y - sign * x) * k)
    };
    let inverse =
        |x: f32, y: f32| WorldPt::new(origin.x + (x - sign * y) * k, origin.y + (y + sign * x) * k);
    let polygon: Vec<_> = points.iter().copied().map(rotate).collect();
    let min = polygon.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max = polygon
        .iter()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let start = (min / 8.0).ceil() * 8.0;
    let count = ((max - start) / 8.0).max(0.0) as usize;
    let mut intersections = Vec::new();
    for n in 0..=count.min(10_000) {
        let y = start + n as f32 * 8.0;
        intersections.clear();
        for i in 0..polygon.len() {
            let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
            if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                intersections.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
        intersections.sort_by(f32::total_cmp);
        for pair in intersections.as_chunks::<2>().0 {
            out.push(Prim::Stroke {
                points: vec![inverse(pair[0], y), inverse(pair[1], y)],
                closed: false,
                width: 1.0,
                color,
            });
        }
    }
}
