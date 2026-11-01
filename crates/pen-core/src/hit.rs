use crate::{Element, Kind, WorldPt, outline};

impl Element {
    /// Uses the canonical shape outline (including rounded corners) or the open path
    /// centreline with stroke reach. `tolerance` is in world points: divide screen tolerance
    /// by camera zoom. Paths never hit an implicit closing edge or their empty interior.
    #[must_use]
    pub fn hit_test(&self, point: WorldPt, tolerance: f32) -> bool {
        if !point.is_finite() || !tolerance.is_finite() {
            return false;
        }
        let bounds = self.kind.bounds();
        if matches!(self.kind, Kind::Text { .. }) {
            return bounds.contains(point);
        }
        let radius = if matches!(self.kind, Kind::Freedraw(_)) {
            self.props.stroke_width * 1.6
        } else {
            self.props.stroke_width / 2.0
        };
        let reach = tolerance.max(0.0) + radius.max(0.0);
        if let Kind::Arrow(path) = &self.kind
            && let Some(head) = path.arrowhead(self.props.stroke_width)
            && head
                .windows(2)
                .any(|s| distance(point, s[0], s[1]) <= reach)
        {
            return true;
        }
        if point.x < bounds.min().x - reach
            || point.x > bounds.max().x + reach
            || point.y < bounds.min().y - reach
            || point.y > bounds.max().y + reach
        {
            return false;
        }
        let points = outline(&self.kind, self.props.edges);
        if self.kind.path().is_some() {
            // Open paths never hit their imaginary closing edge or the box interior.
            return self.props.stroke_width > 0.0
                && points.iter().enumerate().any(|(i, a)| {
                    let b = points.get(i + 1).unwrap_or(a);
                    distance(point, *a, *b) <= reach
                });
        }
        if bounds.is_degenerate() {
            return false;
        }
        let mut inside = false;
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if self.props.stroke_width > 0.0 && distance(point, a, b) <= reach {
                return true;
            }
            if (a.y > point.y) != (b.y > point.y)
                && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
            {
                inside = !inside;
            }
        }
        self.props.background.is_some_and(|color| color.a > 0) && inside
    }
}

fn distance(p: WorldPt, a: WorldPt, b: WorldPt) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / length
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}
