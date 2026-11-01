//! Cached, camera-independent path geometry. perfect-freehand supplies pressure outlines;
//! quadratic midpoint interpolation turns its polygon into smooth curves at every zoom.
use crate::{DrawList, Prim, TOLERANCE};
use pen_core::{Kind, Path, Props, WorldPt};
use perfect_freehand::{InputPoint, StrokeOptions, get_stroke};

pub(crate) fn draw_path(kind: &Kind, path: &Path, props: &Props, out: &mut DrawList) {
    if !(props.stroke_width.is_finite() && props.stroke_width > 0.0) {
        return;
    }
    if !path.origin.is_finite()
        || path.world_points().any(|p| !p.is_finite())
        || path
            .points
            .iter()
            .any(|p| !p.offset.is_finite() || !p.pressure.is_finite())
    {
        return;
    }
    if matches!(kind, Kind::Freedraw(_)) {
        let points = freehand_outline(path, props.stroke_width);
        if points.len() >= 3 {
            out.push(Prim::Fill {
                points,
                color: props.stroke,
            });
        }
    } else {
        let points: Vec<_> = path.world_points().collect();
        if points.len() < 2 {
            return;
        }
        out.push(Prim::Stroke {
            points,
            closed: false,
            width: props.stroke_width,
            color: props.stroke,
        });
    }
}

/// A closed, finite polygon in world points. Explicit pressure prevents simplification from
/// changing stroke weight. A one-sample stroke renders a round dot.
#[must_use]
pub fn freehand_outline(path: &Path, width: f32) -> Vec<WorldPt> {
    if !width.is_finite() || width <= 0.0 || !path.origin.is_finite() {
        return Vec::new();
    }
    let mut input: Vec<_> = path
        .points
        .iter()
        .filter(|p| p.offset.is_finite() && p.pressure.is_finite())
        .map(|p| {
            InputPoint::Array(
                [f64::from(p.offset.x), f64::from(p.offset.y)],
                Some(f64::from(p.pressure.clamp(0.0, 1.0))),
            )
        })
        .collect();
    // This port synthesizes 0.5-pressure samples for exactly two inputs. A midpoint
    // preserves our recorded pressure when simplification reduces a stroke to two points.
    if let [InputPoint::Array(a, pa), InputPoint::Array(b, pb)] = input.as_slice() {
        let mid = InputPoint::Array(
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0],
            Some((pa.unwrap_or(0.5) + pb.unwrap_or(0.5)) / 2.0),
        );
        input.insert(1, mid);
    }
    let outline = get_stroke(
        &input,
        &StrokeOptions {
            size: Some(f64::from(width) * 2.0),
            thinning: Some(0.6),
            smoothing: Some(0.5),
            streamline: Some(0.3),
            simulate_pressure: Some(false),
            last: Some(true),
            ..StrokeOptions::default()
        },
    );
    let points: Vec<_> = outline
        .iter()
        .map(|p| WorldPt::new(p[0] as f32 + path.origin.x, p[1] as f32 + path.origin.y))
        .filter(|p| p.is_finite())
        .collect();
    smooth(&points)
}

fn smooth(points: &[WorldPt]) -> Vec<WorldPt> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mid = |a: WorldPt, b: WorldPt| WorldPt::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let mut out = Vec::new();
    for i in 0..points.len() {
        let c = points[i];
        let a = mid(points[(i + points.len() - 1) % points.len()], c);
        let b = mid(c, points[(i + 1) % points.len()]);
        let curvature = (a.x - 2.0 * c.x + b.x).hypot(a.y - 2.0 * c.y + b.y);
        let n = (curvature / (4.0 * TOLERANCE))
            .sqrt()
            .ceil()
            .clamp(1.0, 128.0) as usize;
        for j in 0..n {
            let t = j as f32 / n as f32;
            let u = 1.0 - t;
            out.push(WorldPt::new(
                u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
            ));
        }
    }
    if let Some(first) = out.first().copied() {
        out.push(first);
    }
    out
}
