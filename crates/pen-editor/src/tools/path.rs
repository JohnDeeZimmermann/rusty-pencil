//! Open paths: a drag makes two endpoints; clicks keep placed vertices and a live tail.
use crate::{CLICK_SLOP, Camera, Modifiers, ScreenPt, Tool};
use pen_core::{Kind, Path, PathPoint, WorldPt};

#[derive(Debug, Clone)]
pub(crate) struct PathDraw {
    tool: Tool,
    path: Path,
    pub(crate) pointer: ScreenPt,
    pub(crate) mods: Modifiers,
    current: WorldPt,
    press: Option<ScreenPt>,
    /// The initial press may become a two-point drag. Later presses place vertices.
    initial: bool,
}

impl PathDraw {
    pub(crate) fn begin(
        tool: Tool,
        at: ScreenPt,
        camera: &Camera,
        mods: Modifiers,
        grid: bool,
    ) -> Self {
        let current =
            crate::properties::snap(camera.screen_to_world(at), grid && tool != Tool::Freedraw);
        Self {
            tool,
            path: Path::from_world(&[current]),
            pointer: at,
            mods,
            current,
            press: Some(at),
            initial: true,
        }
    }

    pub(crate) fn press(&mut self, at: ScreenPt, mods: Modifiers, camera: &Camera, grid: bool) {
        self.update(Some(at), mods, camera, grid);
        self.press = Some(at);
    }

    pub(crate) fn update(
        &mut self,
        at: Option<ScreenPt>,
        mods: Modifiers,
        camera: &Camera,
        grid: bool,
    ) -> bool {
        let old = (self.pointer, self.current, self.mods);
        if let Some(at) = at {
            let p = crate::properties::snap(
                camera.screen_to_world(at),
                grid && self.tool != Tool::Freedraw,
            );
            if p.is_finite() {
                self.pointer = at;
                self.current = p;
            }
        }
        self.mods = mods;
        if self.tool == Tool::Freedraw && self.press.is_some() {
            self.append(self.current, true);
        }
        old != (self.pointer, self.current, self.mods)
    }

    pub(crate) fn release(&mut self) -> bool {
        let Some(press) = self.press.take() else {
            return false;
        };
        let dragged = (self.pointer.x - press.x).hypot(self.pointer.y - press.y) >= CLICK_SLOP;
        if self.tool == Tool::Freedraw {
            return true;
        }
        if !self.initial || dragged {
            self.append(self.tip(), false);
        }
        let finish = self.initial && dragged;
        self.initial = false;
        finish
    }

    fn tip(&self) -> WorldPt {
        let last = self.path.world_points().last().unwrap_or(self.current);
        snap(last, self.current, self.mods.shift)
    }

    fn append(&mut self, p: WorldPt, pressure: bool) {
        let Some(last) = self.path.world_points().last() else {
            return;
        };
        let distance = (p.x - last.x).hypot(p.y - last.y);
        if distance < 0.05 {
            return;
        }
        let previous = self.path.points.last().map_or(0.5, |p| p.pressure);
        // Simulated pressure from sample spacing, as perfect-freehand does. Store it before
        // simplifying, so committing a stroke cannot change its weight.
        let target = (1.0 - distance / 16.0).clamp(0.15, 0.85);
        self.path.points.push(PathPoint {
            offset: WorldPt::new(p.x - self.path.origin.x, p.y - self.path.origin.y),
            pressure: if pressure {
                previous + (target - previous) * 0.2
            } else {
                0.5
            },
        });
    }

    pub(crate) fn backspace(&mut self) {
        if self.path.points.len() > 1 {
            self.path.points.pop();
        }
    }

    pub(crate) fn kind(&self, tail: bool) -> Kind {
        let mut path = self.path.clone();
        if tail && self.tool != Tool::Freedraw {
            let p = self.tip();
            path.points.push(PathPoint {
                offset: WorldPt::new(p.x - path.origin.x, p.y - path.origin.y),
                pressure: 0.5,
            });
        }
        match self.tool {
            Tool::Arrow => Kind::Arrow(path),
            Tool::Freedraw => Kind::Freedraw(path),
            _ => Kind::Line(path),
        }
    }

    pub(crate) fn finish(mut self, zoom: f32, include_tail: bool) -> Option<Kind> {
        if include_tail && self.tool != Tool::Freedraw {
            self.append(self.tip(), false);
        }
        if self.tool == Tool::Freedraw {
            simplify(&mut self.path.points, 0.35 / zoom);
        } else if self.path.points.len() < 2 {
            return None;
        }
        Some(self.kind(false))
    }
}

/// Snapping is relative to the preceding vertex and preserves segment length.
pub(crate) fn snap(from: WorldPt, to: WorldPt, enabled: bool) -> WorldPt {
    if !enabled {
        return to;
    }
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let step = std::f32::consts::PI / 12.0;
    let angle = (dy.atan2(dx) / step).round() * step;
    let len = dx.hypot(dy);
    WorldPt::new(from.x + len * angle.cos(), from.y + len * angle.sin())
}

/// Iterative Ramer–Douglas–Peucker, also preserving pressure extrema. Runs only on commit.
fn simplify(points: &mut Vec<PathPoint>, tolerance: f32) {
    if points.len() < 3 {
        return;
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut ranges = vec![(0, points.len() - 1)];
    while let Some((a, b)) = ranges.pop() {
        let start = points[a];
        let end = points[b];
        let dx = end.offset.x - start.offset.x;
        let dy = end.offset.y - start.offset.y;
        let len = dx * dx + dy * dy;
        let mut furthest = None;
        let mut error = 1.0;
        for (i, p) in points.iter().enumerate().take(b).skip(a + 1) {
            let t = if len > 0.0 {
                ((p.offset.x - start.offset.x) * dx + (p.offset.y - start.offset.y) * dy) / len
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let distance =
                (p.offset.x - start.offset.x - t * dx).hypot(p.offset.y - start.offset.y - t * dy);
            let pressure =
                (p.pressure - (start.pressure + t * (end.pressure - start.pressure))).abs();
            let e = (distance / tolerance).max(pressure / 0.025);
            if e > error {
                error = e;
                furthest = Some(i);
            }
        }
        if let Some(i) = furthest {
            keep[i] = true;
            ranges.push((a, i));
            ranges.push((i, b));
        }
    }
    let mut i = 0;
    points.retain(|_| {
        let yes = keep[i];
        i += 1;
        yes
    });
}
