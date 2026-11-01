//! Point editing keeps the original document untouched until pointer release.
use super::path::snap;
use crate::{Camera, Modifiers, ScreenPt};
use pen_core::{Command, Document, Element, ElementId, Kind, Path, PathPoint, WorldPt};

#[derive(Debug, Clone)]
struct PointDrag {
    before: Element,
    baseline: Path,
    pointer: ScreenPt,
    grab: WorldPt,
    mods: Modifiers,
}

#[derive(Debug, Clone)]
pub(crate) struct PointEditor {
    pub(crate) id: ElementId,
    pub(crate) active: Option<usize>,
    pub(crate) endpoint_only: bool,
    drag: Option<PointDrag>,
    preview: Vec<Element>,
}

impl PointEditor {
    pub(crate) fn new(id: ElementId) -> Self {
        Self {
            id,
            active: None,
            endpoint_only: false,
            drag: None,
            preview: Vec::new(),
        }
    }

    pub(crate) fn path<'a>(&'a self, doc: &'a Document) -> Option<&'a Path> {
        self.preview
            .first()
            .or_else(|| doc.get(self.id))?
            .kind
            .path()
    }

    pub(crate) fn preview(&self) -> &[Element] {
        &self.preview
    }

    /// A vertex moves; a segment click inserts a vertex there and starts the same drag.
    pub(crate) fn press(&mut self, doc: &Document, at: ScreenPt, camera: &Camera) -> bool {
        let Some(before) = doc.get(self.id) else {
            return false;
        };
        let Some(path) = before.kind.path() else {
            return false;
        };
        let p = pen_core::rotate(
            camera.screen_to_world(at),
            before.kind.bounds().centre(),
            -before.angle,
        );
        let reach = 8.0 / camera.zoom();
        let vertex = path
            .world_points()
            .position(|v| (v.x - p.x).hypot(v.y - p.y) <= reach);
        let mut after = before.clone();
        let (index, grab) = if let Some(i) = vertex {
            let v = path.world_point(i).unwrap_or(p);
            (i, WorldPt::new(v.x - p.x, v.y - p.y))
        } else {
            let points: Vec<_> = path.world_points().collect();
            let segment = points
                .windows(2)
                .enumerate()
                .find(|(_, s)| distance(p, s[0], s[1]) <= reach);
            let Some((i, _)) = segment else { return false };
            let path = linear_path(&mut after.kind);
            path.points.insert(
                i + 1,
                PathPoint {
                    offset: WorldPt::new(p.x - path.origin.x, p.y - path.origin.y),
                    pressure: 0.5,
                },
            );
            (i + 1, WorldPt::ORIGIN)
        };
        let baseline = linear_path(&mut after.kind).clone();
        preserve_pivot(&mut after, before.kind.bounds().centre());
        self.active = Some(index);
        self.drag = Some(PointDrag {
            before: before.clone(),
            baseline,
            pointer: at,
            grab,
            mods: Modifiers::NONE,
        });
        self.preview.clear();
        self.preview.push(after);
        true
    }

    pub(crate) fn update(
        &mut self,
        at: Option<ScreenPt>,
        mods: Modifiers,
        camera: &Camera,
        grid: bool,
        doc: &Document,
    ) -> bool {
        let (Some(drag), Some(index), Some(after)) =
            (&mut self.drag, self.active, self.preview.first_mut())
        else {
            return false;
        };
        if let Some(at) = at {
            drag.pointer = at;
        }
        drag.mods = mods;
        let p = pen_core::rotate(
            camera.screen_to_world(drag.pointer),
            drag.before.kind.bounds().centre(),
            -drag.before.angle,
        );
        if !p.is_finite() {
            return false;
        }
        after.angle = drag.before.angle;
        after.bindings = drag.before.bindings;
        let path = linear_path(&mut after.kind);
        path.origin = drag.baseline.origin;
        path.points.clone_from(&drag.baseline.points);
        let mut p =
            crate::properties::snap(WorldPt::new(p.x + drag.grab.x, p.y + drag.grab.y), grid);
        let anchor = if index > 0 { index - 1 } else { 1 };
        if let Some(from) = path.world_point(anchor) {
            p = snap(from, p, mods.shift);
        }
        let offset = WorldPt::new(p.x - path.origin.x, p.y - path.origin.y);
        path.points[index].offset = offset;
        preserve_pivot(after, drag.before.kind.bounds().centre());
        if matches!(after.kind, Kind::Arrow(_)) {
            let end = if index == 0 {
                Some(0)
            } else if index + 1 == drag.baseline.points.len() {
                Some(1)
            } else {
                None
            };
            if let Some(end) = end {
                let point = after.kind.path().and_then(|p| p.world_point(index));
                after.bindings[end] = if mods.ctrl {
                    None
                } else {
                    point.and_then(|p| {
                        pen_core::binding::candidate(
                            doc,
                            pen_core::rotate(p, after.kind.bounds().centre(), after.angle),
                            16.0 / camera.zoom(),
                        )
                    })
                };
            }
            *after = pen_core::binding::fit(after, |id| doc.get(id));
        }
        true
    }

    pub(crate) fn camera_moved(&mut self, camera: &Camera, grid: bool, doc: &Document) -> bool {
        let mods = self.drag.as_ref().map_or(Modifiers::NONE, |d| d.mods);
        self.update(None, mods, camera, grid, doc)
    }

    pub(crate) fn finish(&mut self) -> Option<Command> {
        let drag = self.drag.take()?;
        let after = self.preview.pop()?;
        self.preview.clear();
        (drag.before.kind != after.kind || drag.before.bindings != after.bindings).then_some(
            Command::Patch {
                id: self.id,
                before: drag.before,
                after,
            },
        )
    }

    pub(crate) fn cancel(&mut self) -> bool {
        let cancelled = self.drag.take().is_some();
        self.preview.clear();
        cancelled
    }

    pub(crate) fn delete(&mut self, doc: &Document) -> Option<Command> {
        let index = self.active?;
        let before = doc.get(self.id)?;
        let points = &before.kind.path()?.points;
        if points.len() <= 2 || index >= points.len() {
            return None;
        }
        let mut after = before.clone();
        linear_path(&mut after.kind).points.remove(index);
        preserve_pivot(&mut after, before.kind.bounds().centre());
        self.active = None;
        Some(Command::Patch {
            id: self.id,
            before: before.clone(),
            after,
        })
    }
}

fn linear_path(kind: &mut Kind) -> &mut Path {
    match kind {
        Kind::Line(p) | Kind::Arrow(p) => p,
        _ => unreachable!("point editing begins only for lines and arrows"),
    }
}

fn distance(p: WorldPt, a: WorldPt, b: WorldPt) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx * dx + dy * dy;
    let t = if len > 0.0 {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / len
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}

// Changing a path's bounds changes its rotation pivot. Compensate by a translation so
// untouched vertices stay at their original world positions throughout the edit.
fn preserve_pivot(element: &mut Element, original: WorldPt) {
    if element.angle == 0.0 {
        return;
    }
    let c = element.kind.bounds().centre();
    let rotated = pen_core::rotate(c, original, element.angle);
    let path = linear_path(&mut element.kind);
    path.origin.x += rotated.x - c.x;
    path.origin.y += rotated.y - c.y;
}
