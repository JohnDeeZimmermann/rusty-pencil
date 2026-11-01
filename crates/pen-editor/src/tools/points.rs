//! Point editing keeps the original document untouched until pointer release.
use super::path::snap;
use crate::{Camera, Modifiers, ScreenPt};
use pen_core::{Command, Document, Element, ElementId, Kind, Path, PathPoint, WorldPt};

#[derive(Debug, Clone)]
struct PointDrag {
    before: Element,
    pointer: ScreenPt,
    grab: WorldPt,
    mods: Modifiers,
}

#[derive(Debug, Clone)]
pub(crate) struct PointEditor {
    pub(crate) id: ElementId,
    pub(crate) active: Option<usize>,
    drag: Option<PointDrag>,
    preview: Vec<Element>,
}

impl PointEditor {
    pub(crate) fn new(id: ElementId) -> Self {
        Self {
            id,
            active: None,
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
        let p = camera.screen_to_world(at);
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
        self.active = Some(index);
        self.drag = Some(PointDrag {
            before: before.clone(),
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
        let p = camera.screen_to_world(drag.pointer);
        if !p.is_finite() {
            return false;
        }
        let path = linear_path(&mut after.kind);
        let mut p = WorldPt::new(p.x + drag.grab.x, p.y + drag.grab.y);
        let anchor = if index > 0 { index - 1 } else { 1 };
        if let Some(from) = path.world_point(anchor) {
            p = snap(from, p, mods.shift);
        }
        let offset = WorldPt::new(p.x - path.origin.x, p.y - path.origin.y);
        if path.points[index].offset == offset {
            return false;
        }
        path.points[index].offset = offset;
        true
    }

    pub(crate) fn camera_moved(&mut self, camera: &Camera) -> bool {
        let mods = self.drag.as_ref().map_or(Modifiers::NONE, |d| d.mods);
        self.update(None, mods, camera)
    }

    pub(crate) fn finish(&mut self) -> Option<Command> {
        let drag = self.drag.take()?;
        let after = self.preview.pop()?;
        self.preview.clear();
        (drag.before.kind != after.kind).then_some(Command::Patch {
            id: self.id,
            before: drag.before,
            after,
        })
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
