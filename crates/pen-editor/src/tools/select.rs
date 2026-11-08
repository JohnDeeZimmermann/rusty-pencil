//! Selection gesture lifecycle. Original snapshots, live previews, and cancellation belong
//! to the gesture; the editor executes the resulting command through history on release.
use super::transform::resize;
use crate::{CLICK_SLOP, Camera, Handle, Modifiers, ScreenPt, Selection};
use pen_core::{Command, Document, Element, WorldPt, WorldRect};

#[derive(Debug, Clone)]
enum Action {
    Move(Vec<Element>),
    Resize {
        originals: Vec<Element>,
        bounds: WorldRect,
        handle: Handle,
    },
    Rotate(Vec<Element>, WorldRect),
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
    duplicate: bool,
    duplicates: Vec<Element>,
}

impl SelectionDrag {
    /// The editor rejects nonfinite presses before starting a gesture. Selection changes
    /// immediately; geometry remains in the document until the returned edit is executed.
    pub(crate) fn begin(
        doc: &mut Document,
        selection: &mut Selection,
        camera: &Camera,
        at: ScreenPt,
        mods: Modifiers,
        scope: Option<pen_core::GroupId>,
    ) -> Self {
        let origin = camera.screen_to_world(at);
        let before = selection.clone();
        let handle = if selection.elements(doc).any(|e| e.locked) {
            None
        } else {
            selection
                .bounds(doc)
                .and_then(|r| Handle::hit(r, origin, camera.zoom()))
        };
        let handle = handle.filter(|handle| {
            if *handle == Handle::Rotate
                && selection
                    .elements(doc)
                    .any(|e| matches!(e.kind, pen_core::Kind::Frame { .. }))
            {
                return false;
            }
            if selection.len() == 1
                && selection
                    .elements(doc)
                    .any(|e| matches!(e.kind, pen_core::Kind::Arrow(_)))
                && *handle != Handle::Rotate
            {
                return false;
            }
            selection.len() == 1 || !selection.bounds(doc).is_some_and(|b| b.contains(origin))
        });
        let hit = doc
            .pick(origin, 5.0 / camera.zoom(), true)
            .map(|e| pen_core::bound::owner(doc, e.id()));
        let action = if let Some(handle) = handle {
            let originals = selection.elements(doc).cloned().collect();
            let bounds = selection.bounds(doc).unwrap_or_default();
            if handle == Handle::Rotate {
                Action::Rotate(originals, bounds)
            } else {
                Action::Resize {
                    originals,
                    bounds,
                    handle,
                }
            }
        } else if let Some(id) = hit {
            if mods.shift {
                if selection.contains(id) {
                    let mut members = Selection::default();
                    members.expand_group(doc, id, scope);
                    for id in members.ids() {
                        selection.toggle(id);
                    }
                } else {
                    selection.expand_group(doc, id, scope);
                }
                if !selection.contains(id) {
                    Action::Toggle
                } else {
                    Action::Move(selection.elements(doc).cloned().collect())
                }
            } else {
                if !selection.contains(id) {
                    selection.clear();
                    selection.expand_group(doc, id, scope);
                }
                Action::Move(selection.elements(doc).cloned().collect())
            }
        } else if !mods.shift
            && !selection.elements(doc).any(|e| e.locked)
            && selection.bounds(doc).is_some_and(|r| r.contains(origin))
        {
            // Once selected, a hollow shape can be grabbed by its interior.
            Action::Move(selection.elements(doc).cloned().collect())
        } else {
            if !mods.shift {
                selection.clear();
            }
            Action::Box
        };
        let duplicate = mods.alt && matches!(action, Action::Move(_));
        let mut drag = Self {
            action,
            duplicate,
            duplicates: Vec::new(),
            phase: Phase::Pressed,
            before,
            origin,
            current: origin,
            start: at,
            pointer: at,
            mods,
            preview: Vec::new(),
        };
        if duplicate {
            drag.prepare_duplicates(doc);
        }
        drag
    }

    fn prepare_duplicates(&mut self, doc: &mut Document) {
        if !self.duplicates.is_empty() {
            return;
        }
        if let Action::Move(originals) = &self.action {
            let mut originals = originals.clone();
            let selected: Vec<_> = originals.iter().map(Element::id).collect();
            originals.extend(
                doc.elements()
                    .iter()
                    .filter(|e| {
                        !selected.contains(&e.id())
                            && (e.frame_id.is_some_and(|id| selected.contains(&id))
                                || e.kind
                                    .container()
                                    .is_some_and(|c| selected.contains(&c.id())))
                    })
                    .cloned(),
            );
            originals.sort_by(|a, b| a.z_key.cmp(&b.z_key));
            let ids: std::collections::BTreeMap<_, _> =
                originals.iter().map(|e| (e.id(), doc.alloc_id())).collect();
            let mut next_group = doc
                .elements()
                .iter()
                .flat_map(|e| &e.group_ids)
                .map(|g| g.0)
                .max()
                .unwrap_or(0);
            let mut groups = std::collections::BTreeMap::new();
            for e in &mut originals {
                *e = e.copied(ids[&e.id()]);
                e.frame_id = e.frame_id.and_then(|id| ids.get(&id).copied());
                if let Some(pen_core::Link::Element(id)) = e.link {
                    e.link = ids.get(&id).copied().map(pen_core::Link::Element);
                }
                for g in &mut e.group_ids {
                    *g = *groups.entry(*g).or_insert_with(|| {
                        next_group += 1;
                        pen_core::GroupId(next_group)
                    });
                }
                for link in &mut e.bindings {
                    *link = link.and_then(|mut b| {
                        b.element = *ids.get(&b.element)?;
                        Some(b)
                    });
                }
                if let pen_core::Kind::Text { container, .. } = &mut e.kind {
                    *container = container.and_then(|c| {
                        ids.get(&c.id()).map(|id| match c {
                            pen_core::Container::Shape(_) => pen_core::Container::Shape(*id),
                            pen_core::Container::Path(_) => pen_core::Container::Path(*id),
                        })
                    });
                }
            }
            self.duplicates = originals;
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
        doc: &mut Document,
        polish: &mut crate::polish::Polish,
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
        self.duplicate = mods.alt && matches!(self.action, Action::Move(_));
        if self.duplicate {
            self.prepare_duplicates(doc);
        }
        self.preview.clear();
        let delta = WorldPt::new(
            self.current.x - self.origin.x,
            self.current.y - self.origin.y,
        );
        match &self.action {
            Action::Move(originals) => {
                let originals = if self.duplicate {
                    &self.duplicates
                } else {
                    originals
                };
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
                let delta = if polish.modes.snap_objects && !mods.ctrl {
                    polish
                        .snapping
                        .adjust(originals, delta, camera.zoom(), &mut polish.guides)
                } else {
                    polish.guides.clear();
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
            Action::Resize {
                originals,
                bounds,
                handle,
            } => {
                let delta = if grid {
                    let bounds = *bounds;
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
                let mods = if originals.len() == 1
                    && matches!(originals[0].kind, pen_core::Kind::Text { .. })
                {
                    Modifiers {
                        shift: true,
                        ..mods
                    }
                } else {
                    mods
                };
                let target = resize(*bounds, *handle, delta, mods, camera.zoom());
                for original in originals {
                    let b = original.kind.bounds();
                    let map = |p: WorldPt| {
                        WorldPt::new(
                            target.min().x
                                + (p.x - bounds.min().x) * target.width()
                                    / bounds.width().max(0.001),
                            target.min().y
                                + (p.y - bounds.min().y) * target.height()
                                    / bounds.height().max(0.001),
                        )
                    };
                    self.preview.push(
                        original.with_bounds(WorldRect::from_corners(map(b.min()), map(b.max()))),
                    );
                }
            }
            Action::Rotate(originals, bounds) => {
                let c = bounds.centre();
                let mut angle = (self.current.y - c.y).atan2(self.current.x - c.x)
                    - (self.origin.y - c.y).atan2(self.origin.x - c.x);
                if mods.shift {
                    let step = std::f32::consts::PI / 12.0;
                    let base = if originals.len() == 1 {
                        originals[0].angle
                    } else {
                        0.0
                    };
                    angle = ((angle + base) / step).round() * step - base;
                }
                self.preview
                    .extend(originals.iter().map(|e| e.rotated(c, angle)));
            }
            Action::Box | Action::Toggle => {}
        }
        if matches!(self.action, Action::Move(_)) {
            for i in 0..self.preview.len() {
                if matches!(self.preview[i].kind, pen_core::Kind::Frame { .. })
                    || self.preview[i]
                        .frame_id
                        .is_some_and(|id| self.preview.iter().any(|e| e.id() == id))
                {
                    continue;
                }
                self.preview[i].frame_id = doc
                    .frame_at(self.preview[i].bounds().centre())
                    .map(Element::id);
            }
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
                if !element.locked
                    && element.kind.container().is_none()
                    && pen_core::frames::visible_bounds(doc, element)
                        .is_some_and(|visible| bounds.intersects(visible))
                {
                    selection.expand_group(doc, element.id(), None);
                }
            }
            return None;
        }
        if self.duplicate && !self.preview.is_empty() {
            selection.clear();
            for e in &self.preview {
                if e.kind.container().is_none() {
                    selection.insert(e.id());
                }
            }
            let commands = self
                .preview
                .into_iter()
                .enumerate()
                .map(|(i, element)| Command::Add {
                    element,
                    at: doc.elements().len() + i,
                })
                .collect();
            return Some((Command::Group(commands), self.before));
        }
        let commands = self
            .preview
            .iter()
            .filter_map(|after| {
                let before = doc.get(after.id())?;
                (before != after).then(|| Command::Patch {
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
