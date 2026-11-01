//! Selection gesture lifecycle. Original snapshots, live previews, and cancellation belong
//! to the gesture; the editor executes the resulting command through history on release.
use super::transform::resize;
use crate::{CLICK_SLOP, Camera, Handle, Modifiers, ScreenPt, Selection};
use pen_core::{Command, Document, Element, WorldPt, WorldRect};

#[derive(Debug, Clone)]
enum Action {
    Move(Vec<Element>),
    Resize {
        original: Element,
        handle: Handle,
    },
    Box,
    /// Shift removes the clicked element without starting a move.
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Pressed,
    Dragging,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectionDrag {
    action: Action,
    phase: Phase,
    before: Selection,
    origin: WorldPt,
    current: WorldPt,
    start: ScreenPt,
    pointer: ScreenPt,
    mods: Modifiers,
    preview: Vec<Element>,
}

impl SelectionDrag {
    /// The editor rejects nonfinite presses before starting a gesture. Selection changes
    /// immediately; geometry remains in the document until the returned edit is executed.
    pub(crate) fn begin(
        doc: &Document,
        selection: &mut Selection,
        camera: &Camera,
        at: ScreenPt,
        mods: Modifiers,
        _grid: bool,
    ) -> Self {
        let origin = camera.screen_to_world(at);
        let before = selection.clone();
        let handle = if selection.len() == 1 {
            selection
                .bounds(doc)
                .and_then(|r| Handle::hit(r, origin, camera.zoom()))
        } else {
            None
        };
        let hit = doc
            .elements()
            .iter()
            .rev()
            .find(|e| e.hit_test(origin, 5.0 / camera.zoom()))
            .map(|e| pen_core::bound::owner(doc, e.id()));
        let action = if let Some(handle) = handle {
            selection
                .elements(doc)
                .next()
                .cloned()
                .map(|original| Action::Resize { original, handle })
                .unwrap_or(Action::Box)
        } else if let Some(id) = hit {
            if mods.shift {
                selection.toggle(id);
                if !selection.contains(id) {
                    Action::Toggle
                } else {
                    Action::Move(selection.elements(doc).cloned().collect())
                }
            } else {
                if !selection.contains(id) {
                    *selection = Selection::only(id);
                }
                Action::Move(selection.elements(doc).cloned().collect())
            }
        } else if !mods.shift && selection.bounds(doc).is_some_and(|r| r.contains(origin)) {
            // Once selected, a hollow shape can be grabbed by its interior.
            Action::Move(selection.elements(doc).cloned().collect())
        } else {
            if !mods.shift {
                selection.clear();
            }
            Action::Box
        };
        Self {
            action,
            phase: Phase::Pressed,
            before,
            origin,
            current: origin,
            start: at,
            pointer: at,
            mods,
            preview: Vec::new(),
        }
    }

    /// Updates from original snapshots, including when only modifiers or the camera change.
    /// Returns whether live feedback needs presenting; never mutates the document.
    pub(crate) fn update(
        &mut self,
        at: Option<ScreenPt>,
        mods: Modifiers,
        camera: &Camera,
        grid: bool,
    ) -> bool {
        let old = (self.pointer, self.current, self.mods);
        if let Some(at) = at {
            let current = camera.screen_to_world(at);
            if current.is_finite() {
                self.current = current;
                self.pointer = at;
            }
        }
        self.mods = mods;
        if old == (self.pointer, self.current, self.mods) {
            return false;
        }
        if (self.pointer.x - self.start.x).hypot(self.pointer.y - self.start.y) >= CLICK_SLOP {
            self.phase = Phase::Dragging;
        }
        if self.phase == Phase::Pressed {
            return false;
        }
        self.preview.clear();
        let delta = WorldPt::new(
            self.current.x - self.origin.x,
            self.current.y - self.origin.y,
        );
        match &self.action {
            Action::Move(originals) => {
                let delta = if grid {
                    let bounds = originals
                        .iter()
                        .map(|e| e.kind.bounds())
                        .reduce(WorldRect::union);
                    bounds.map_or(delta, |b| {
                        let p = b.min();
                        let next = crate::properties::snap(
                            WorldPt::new(p.x + delta.x, p.y + delta.y),
                            true,
                        );
                        WorldPt::new(next.x - p.x, next.y - p.y)
                    })
                } else {
                    delta
                };
                let (dx, dy) = if mods.shift {
                    if delta.x.abs() >= delta.y.abs() {
                        (delta.x, 0.0)
                    } else {
                        (0.0, delta.y)
                    }
                } else {
                    (delta.x, delta.y)
                };
                self.preview.extend(
                    originals
                        .iter()
                        .map(|e| e.with_bounds(e.kind.bounds().translated(dx, dy))),
                );
            }
            Action::Resize { original, handle } => {
                let delta = if grid {
                    let bounds = original.kind.bounds();
                    let (hx, hy) = handle.axes();
                    let anchor = WorldPt::new(
                        bounds.centre().x + hx * bounds.width() / 2.0,
                        bounds.centre().y + hy * bounds.height() / 2.0,
                    );
                    let target = crate::properties::snap(
                        WorldPt::new(anchor.x + delta.x, anchor.y + delta.y),
                        true,
                    );
                    WorldPt::new(
                        if hx == 0.0 { 0.0 } else { target.x - anchor.x },
                        if hy == 0.0 { 0.0 } else { target.y - anchor.y },
                    )
                } else {
                    delta
                };
                self.preview.push(original.with_bounds(resize(
                    original.kind.bounds(),
                    *handle,
                    delta,
                    if matches!(original.kind, pen_core::Kind::Text { .. }) {
                        Modifiers {
                            shift: true,
                            ..mods
                        }
                    } else {
                        mods
                    },
                    camera.zoom(),
                )));
            }
            Action::Box | Action::Toggle => {}
        }
        true
    }

    pub(crate) fn preview(&self) -> &[Element] {
        &self.preview
    }

    /// The camera can re-evaluate the same screen pointer against its new transform.
    pub(crate) fn pointer(&self) -> (ScreenPt, Modifiers) {
        (self.pointer, self.mods)
    }

    pub(crate) fn marquee(&self) -> Option<WorldRect> {
        (matches!(self.action, Action::Box) && self.phase == Phase::Dragging)
            .then(|| WorldRect::from_corners(self.origin, self.current))
    }

    pub(crate) fn cancel(self, selection: &mut Selection) {
        *selection = self.before;
    }

    /// Finalizes membership or returns one grouped edit with its pre-press selection. The
    /// editor owns history; consuming the gesture also discards every live preview.
    pub(crate) fn finish(
        self,
        doc: &Document,
        selection: &mut Selection,
    ) -> Option<(Command, Selection)> {
        if let Some(bounds) = self.marquee() {
            // Membership is scanned once on release, never on each animation frame.
            *selection = if self.mods.shift {
                self.before
            } else {
                Selection::default()
            };
            for element in doc.elements() {
                // Bound text is reached through its container.
                if element.kind.container().is_none() && bounds.intersects(element.kind.bounds()) {
                    selection.insert(element.id());
                }
            }
            return None;
        }
        let commands = self
            .preview
            .iter()
            .filter_map(|after| {
                let before = doc.get(after.id())?;
                (before.kind != after.kind).then(|| Command::Patch {
                    id: after.id(),
                    before: before.clone(),
                    after: after.clone(),
                })
            })
            .collect::<Vec<_>>();
        if commands.is_empty() {
            None
        } else {
            Some((Command::Group(commands), self.before))
        }
    }
}
