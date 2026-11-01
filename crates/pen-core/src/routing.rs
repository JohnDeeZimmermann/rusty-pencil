//! Deterministic orthogonal routing on the endpoint shapes' visibility grid.
#![expect(
    clippy::float_cmp,
    reason = "Grid coordinates are copied exactly, never approximated"
)]
use crate::{Path, WorldPt, WorldRect};

/// Routes between endpoint escape points without crossing the open interiors of `obstacles`.
/// Grid coordinates come from obstacle edges; cost is Manhattan distance in world points.
#[must_use]
pub fn elbow(a: WorldPt, b: WorldPt, obstacles: &[WorldRect]) -> Path {
    let escape = |p: WorldPt, other: WorldPt| {
        let Some(r) = obstacles.iter().find(|r| r.contains(p)) else {
            return p;
        };
        let c = r.centre();
        let dx = p.x - c.x;
        let dy = p.y - c.y;
        if dx.abs() / r.width().max(1.0) >= dy.abs() / r.height().max(1.0) {
            WorldPt::new(
                if dx > 0.0 || (dx == 0.0 && other.x >= p.x) {
                    r.max().x + 20.0
                } else {
                    r.min().x - 20.0
                },
                p.y,
            )
        } else {
            WorldPt::new(
                p.x,
                if dy > 0.0 {
                    r.max().y + 20.0
                } else {
                    r.min().y - 20.0
                },
            )
        }
    };
    let start = escape(a, b);
    let end = escape(b, a);
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for r in obstacles {
        xs.extend([r.min().x - 20.0, r.max().x + 20.0]);
        ys.extend([r.min().y - 20.0, r.max().y + 20.0]);
    }
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    let nodes: Vec<_> = ys
        .iter()
        .flat_map(|y| xs.iter().map(move |x| WorldPt::new(*x, *y)))
        .collect();
    let first = nodes.iter().position(|p| *p == start).unwrap_or(0);
    let last = nodes.iter().position(|p| *p == end).unwrap_or(0);
    let mut costs = vec![f32::INFINITY; nodes.len()];
    let mut previous = vec![None; nodes.len()];
    let mut visited = vec![false; nodes.len()];
    costs[first] = 0.0;
    for _ in 0..nodes.len() {
        let Some(i) = (0..nodes.len())
            .filter(|i| !visited[*i])
            .min_by(|i, j| costs[*i].total_cmp(&costs[*j]))
        else {
            break;
        };
        if !costs[i].is_finite() || i == last {
            break;
        }
        visited[i] = true;
        for j in 0..nodes.len() {
            let (p, q) = (nodes[i], nodes[j]);
            if visited[j]
                || (p.x != q.x && p.y != q.y)
                || obstacles.iter().any(|r| crosses(p, q, *r))
            {
                continue;
            }
            let cost = costs[i] + (p.x - q.x).abs() + (p.y - q.y).abs();
            if cost < costs[j] {
                costs[j] = cost;
                previous[j] = Some(i);
            }
        }
    }
    let mut route = vec![end];
    let mut at = last;
    while at != first {
        let Some(next) = previous[at] else {
            return Path::from_world(&[a, start, WorldPt::new(start.x, end.y), end, b]);
        };
        at = next;
        route.push(nodes[at]);
    }
    route.reverse();
    route.insert(0, a);
    route.push(b);
    route.dedup();
    let mut clean: Vec<WorldPt> = Vec::new();
    for p in route {
        while clean.len() >= 2 {
            let (u, v) = (clean[clean.len() - 2], clean[clean.len() - 1]);
            if (u.x == v.x && v.x == p.x) || (u.y == v.y && v.y == p.y) {
                clean.pop();
            } else {
                break;
            }
        }
        clean.push(p);
    }
    Path::from_world(&clean)
}

fn crosses(a: WorldPt, b: WorldPt, r: WorldRect) -> bool {
    if a.x == b.x {
        a.x > r.min().x && a.x < r.max().x && a.y.min(b.y) < r.max().y && a.y.max(b.y) > r.min().y
    } else {
        a.y > r.min().y && a.y < r.max().y && a.x.min(b.x) < r.max().x && a.x.max(b.x) > r.min().x
    }
}

/// Catmull–Rom interpolating cubic Béziers, flattened at the shared world tolerance.
#[must_use]
pub fn curved(path: &Path) -> Path {
    let points: Vec<_> = path.world_points().collect();
    if points.len() < 3 {
        return path.clone();
    }
    let mut out = Vec::new();
    for i in 0..points.len() - 1 {
        let (p0, p1, p2, p3) = (
            points[i.saturating_sub(1)],
            points[i],
            points[i + 1],
            points[(i + 2).min(points.len() - 1)],
        );
        let c1 = WorldPt::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
        let c2 = WorldPt::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
        let curvature = (p1.x - 2.0 * c1.x + c2.x)
            .hypot(p1.y - 2.0 * c1.y + c2.y)
            .max((c1.x - 2.0 * c2.x + p2.x).hypot(c1.y - 2.0 * c2.y + p2.y));
        let n = (3.0 * curvature / (4.0 * crate::TOLERANCE))
            .sqrt()
            .ceil()
            .clamp(1.0, 4096.0) as usize;
        for j in 0..n {
            let t = j as f32 / n as f32;
            let u = 1.0 - t;
            out.push(WorldPt::new(
                u * u * u * p1.x
                    + 3.0 * u * u * t * c1.x
                    + 3.0 * u * t * t * c2.x
                    + t * t * t * p2.x,
                u * u * u * p1.y
                    + 3.0 * u * u * t * c1.y
                    + 3.0 * u * t * t * c2.y
                    + t * t * t * p2.y,
            ));
        }
    }
    if let Some(last) = points.last() {
        out.push(*last);
    }
    Path::from_world(&out)
}
