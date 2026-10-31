use crate::{Element, WorldPt, outline};

impl Element {
    /// Uses the same flattened outline as rendering, including rounded corners. `tolerance`
    /// is in world points: callers divide their screen tolerance by camera zoom.
    #[must_use]
    pub fn hit_test(&self, point: WorldPt, tolerance: f32) -> bool {
        if !point.is_finite() || !tolerance.is_finite() || self.kind.bounds().is_degenerate() {
            return false;
        }
        let bounds = self.kind.bounds();
        let reach = tolerance.max(0.0) + self.props.stroke_width.max(0.0) / 2.0;
        if point.x < bounds.min().x - reach
            || point.x > bounds.max().x + reach
            || point.y < bounds.min().y - reach
            || point.y > bounds.max().y + reach
        {
            return false;
        }
        let points = outline(&self.kind, self.props.edges);
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
