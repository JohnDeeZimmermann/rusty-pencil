use crate::{Element, Kind, WorldPt, outline};

impl Element {
    /// Uses the canonical shape outline (including rounded corners) or the open path
    /// centreline with stroke reach. `tolerance` is in world points: divide screen tolerance
    /// by camera zoom. Paths never hit an implicit closing edge or their empty interior.
    #[must_use]
    pub fn hit_test(&self, point: WorldPt, tolerance: f32) -> bool {
        self.hit(point, tolerance, false)
    }

    /// [`Self::hit_test`], but a closed shape's interior counts even without a fill: where a
    /// double-click lands to write inside a shape.
    #[must_use]
    pub fn hit_test_interior(&self, point: WorldPt, tolerance: f32) -> bool {
        self.hit(point, tolerance, true)
    }

    fn hit(&self, point: WorldPt, tolerance: f32, interior: bool) -> bool {
        if !point.is_finite() || !tolerance.is_finite() {
            return false;
        }
        if let Kind::Arrow(path) = &self.kind
            && self.props.arrow_type == crate::ArrowType::Curved
        {
            let mut flattened = self.clone();
            flattened.kind = Kind::Arrow(crate::routing::curved(path));
            flattened.angle = 0.0;
            return flattened.hit_geometry(
                crate::rotate(point, self.kind.bounds().centre(), -self.angle),
                tolerance,
                interior,
            );
        }
        self.hit_geometry(point, tolerance, interior)
    }

    fn hit_geometry(&self, point: WorldPt, tolerance: f32, interior: bool) -> bool {
        let bounds = self.kind.bounds();
        let point = crate::rotate(point, bounds.centre(), -self.angle);
        if matches!(self.kind, Kind::Text { .. } | Kind::Image { .. }) {
            return bounds.contains(point);
        }
        if matches!(self.kind, Kind::Frame { .. })
            && point.x >= bounds.min().x
            && point.x <= bounds.max().x
            && point.y >= bounds.min().y - 24.0
            && point.y <= bounds.min().y
        {
            return true;
        }
        let radius = if matches!(self.kind, Kind::Freedraw(_)) {
            self.props.stroke_width * 1.6
        } else {
            self.props.stroke_width / 2.0
        };
        let reach = tolerance.max(0.0) + radius.max(0.0);
        if let Kind::Arrow(path) = &self.kind
            && self.props.stroke_width > 0.0
        {
            for head in crate::arrowheads(path, &self.props) {
                if head
                    .points
                    .windows(2)
                    .any(|s| distance(point, s[0], s[1]) <= reach)
                {
                    return true;
                }
                if head.closed {
                    if let (Some(a), Some(b)) = (head.points.first(), head.points.last())
                        && distance(point, *a, *b) <= reach
                    {
                        return true;
                    }
                    if head.filled && inside_polygon(point, &head.points) {
                        return true;
                    }
                }
            }
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
        (interior || self.props.background.is_some_and(|color| color.a > 0)) && inside
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

fn inside_polygon(point: WorldPt, polygon: &[WorldPt]) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
    }
    inside
}

impl crate::Document {
    /// Highest visible element at `point`, using the command-maintained spatial index.
    pub fn hit(
        &self,
        point: WorldPt,
        tolerance: f32,
        interior: bool,
        unlocked: bool,
    ) -> Option<&Element> {
        let area = crate::WorldRect::from_centre(point, tolerance.max(0.0), tolerance.max(0.0));
        let mut top = None;
        self.spatial().visit(area, |at| {
            let e = &self.elements()[at];
            if top.is_some_and(|index| at <= index) || !self.hittable(e, point, unlocked) {
                return;
            }
            if if interior {
                e.hit_test_interior(point, tolerance)
            } else {
                e.hit_test(point, tolerance)
            } {
                top = Some(at);
            }
        });
        top.and_then(|at| self.elements().get(at))
    }

    /// The element a click at `point` selects. Visible ink (stroke, fill, text, image) wins,
    /// topmost first. Failing that, the smallest hollow rectangle, diamond or ellipse whose
    /// interior holds the point: nothing covers a hollow interior, so the tightest enclosing
    /// shape is the one the user means, whatever its z-order. Frames select by their edge
    /// and label only, so a press inside one can still start a box selection.
    pub fn pick(&self, point: WorldPt, tolerance: f32, unlocked: bool) -> Option<&Element> {
        let area = crate::WorldRect::from_centre(point, tolerance.max(0.0), tolerance.max(0.0));
        let mut ink: Option<usize> = None;
        let mut hollow: Option<(f32, usize)> = None;
        self.spatial().visit(area, |at| {
            let e = &self.elements()[at];
            if ink.is_some_and(|index| at <= index) || !self.hittable(e, point, unlocked) {
                return;
            }
            if e.hit_test(point, tolerance) {
                ink = Some(at);
            } else if ink.is_none()
                && matches!(e.kind, Kind::Rect(_) | Kind::Diamond(_) | Kind::Ellipse(_))
                && e.hit_test_interior(point, tolerance)
            {
                let size = e.kind.bounds().width() * e.kind.bounds().height();
                // Equal sizes fall back to z-order, so the result never depends on visit order.
                if hollow
                    .is_none_or(|(best, index)| size.total_cmp(&best).then(index.cmp(&at)).is_lt())
                {
                    hollow = Some((size, at));
                }
            }
        });
        ink.or(hollow.map(|(_, at)| at))
            .and_then(|at| self.elements().get(at))
    }

    fn hittable(&self, e: &Element, point: WorldPt, unlocked: bool) -> bool {
        !(unlocked
            && (e.locked
                || e.frame_id
                    .and_then(|id| self.get(id))
                    .is_some_and(|frame| frame.locked)
                || self
                    .get(crate::bound::owner(self, e.id()))
                    .is_some_and(|e| e.locked)))
            && crate::frames::visible_at(self, e, point)
    }
}

impl crate::Document {
    /// Topmost frame containing a centre point, for live drag ownership feedback.
    pub fn frame_at(&self, point: WorldPt) -> Option<&Element> {
        let mut top = None;
        self.spatial()
            .visit(crate::WorldRect::from_corners(point, point), |at| {
                let e = &self.elements()[at];
                if top.is_none_or(|i| at > i)
                    && matches!(e.kind, Kind::Frame { .. })
                    && e.kind.bounds().contains(point)
                {
                    top = Some(at);
                }
            });
        top.and_then(|at| self.elements().get(at))
    }
}
