//! Canonical clean outlines, shared by hit testing and rendering.

use std::f32::consts::{PI, TAU};

use crate::{Edges, Kind, WorldPt, WorldRect};

/// Maximum outline flattening error, in world points.
pub const TOLERANCE: f32 = 0.02;

/// Excalidraw's adaptive corner radius: a quarter of `side` up to 128 pt, then a fixed 32 pt.
fn adaptive_radius(side: f32) -> f32 {
    (side * 0.25).min(32.0)
}

/// The closed outline of `kind`, without repeating the first point at the end.
pub fn outline(kind: &Kind, edges: Edges) -> Vec<WorldPt> {
    match *kind {
        Kind::Frame { bounds: r, .. }
        | Kind::Image { bounds: r, .. }
        | Kind::Text { bounds: r, .. } => vec![
            r.min(),
            WorldPt::new(r.max().x, r.min().y),
            r.max(),
            WorldPt::new(r.min().x, r.max().y),
        ],
        Kind::Rect(r) => {
            let (a, b) = (r.min(), r.max());
            let corners = [a, WorldPt::new(b.x, a.y), b, WorldPt::new(a.x, b.y)];
            let radius = match edges {
                Edges::Sharp => 0.0,
                Edges::Round => adaptive_radius(r.width().min(r.height())),
            };
            rounded_polygon(&corners, radius)
        }
        Kind::Diamond(r) => {
            let (a, b, c) = (r.min(), r.max(), r.centre());
            let corners = [
                WorldPt::new(c.x, a.y),
                WorldPt::new(b.x, c.y),
                WorldPt::new(c.x, b.y),
                WorldPt::new(a.x, c.y),
            ];
            // Excalidraw sizes diamond corners from the half extents.
            let radius = match edges {
                Edges::Sharp => 0.0,
                Edges::Round => adaptive_radius(r.width().min(r.height()) / 2.0),
            };
            rounded_polygon(&corners, radius)
        }
        Kind::Ellipse(r) => ellipse(r),
        Kind::Line(ref p) | Kind::Arrow(ref p) | Kind::Freedraw(ref p) => {
            p.world_points().collect()
        }
    }
}

/// A convex polygon with each corner replaced by a quadratic curve that starts and ends
/// `radius` along the two edges (capped at half of each edge), as Excalidraw rounds corners.
fn rounded_polygon(corners: &[WorldPt], radius: f32) -> Vec<WorldPt> {
    if radius <= 0.0 {
        return corners.to_vec();
    }
    let n = corners.len();
    let mut out = Vec::new();
    for i in 0..n {
        let prev = corners[(i + n - 1) % n];
        let at = corners[i];
        let next = corners[(i + 1) % n];
        let start = toward(at, prev, radius);
        let end = toward(at, next, radius);
        quad(&mut out, start, at, end);
    }
    out
}

/// The point `distance` from `from` toward `to`, at most halfway there.
fn toward(from: WorldPt, to: WorldPt, distance: f32) -> WorldPt {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let len = dx.hypot(dy);
    if len <= f32::EPSILON {
        return from;
    }
    let t = (distance / len).min(0.5);
    WorldPt::new(from.x + dx * t, from.y + dy * t)
}

/// Appends a flattened quadratic Bézier from `a` (included) to `b` (included).
fn quad(out: &mut Vec<WorldPt>, a: WorldPt, c: WorldPt, b: WorldPt) {
    let length = (c.x - a.x).hypot(c.y - a.y) + (b.x - c.x).hypot(b.y - c.y);
    let n = segments(length);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        out.push(WorldPt::new(
            u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
            u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
        ));
    }
}

/// Segments for a curve whose control polygon is `length` long, within [`TOLERANCE`].
fn segments(length: f32) -> usize {
    // Bounded, so the cast can't truncate meaningfully.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = ((length / TOLERANCE).sqrt() * 0.5).ceil().clamp(1.0, 128.0) as usize;
    n
}

/// The inscribed ellipse, with enough points that each chord stays within [`TOLERANCE`] of
/// the true curve at the larger radius.
fn ellipse(r: WorldRect) -> Vec<WorldPt> {
    let c = r.centre();
    let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
    let big = rx.max(ry);
    let n = if big <= TOLERANCE {
        4
    } else {
        // A chord of angle θ on radius R sags R(1 - cos(θ/2)).
        let step = 2.0 * (1.0 - TOLERANCE / big).clamp(-1.0, 1.0).acos();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (TAU / step).ceil().clamp(16.0, 1024.0) as usize;
        n
    };
    (0..n)
        .map(|i| {
            // Start at the top, like Excalidraw, so a dashed outline (M5) begins there.
            let a = i as f32 / n as f32 * TAU - PI / 2.0;
            WorldPt::new(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect()
}
