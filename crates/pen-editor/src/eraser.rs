//! A stroke previews hidden elements and commits removals once, on release.
use crate::{Editor, Modifiers, Redraw, ScreenPt};
use pen_core::{Command, Element, WorldPt, bound};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct Eraser {
    erased: BTreeMap<pen_core::ElementId, Element>,
    pub(crate) preview: Vec<Element>,
    previous: Option<WorldPt>,
    index: Option<crate::eraser_index::EraserIndex>,
    candidates: Vec<pen_core::ElementId>,
    followers: BTreeMap<pen_core::ElementId, Vec<pen_core::ElementId>>,
}

impl Eraser {
    pub(crate) fn new(doc: &pen_core::Document) -> Self {
        let mut stroke = Self {
            index: crate::eraser_index::EraserIndex::new(doc),
            ..Self::default()
        };
        for e in doc.elements() {
            if let Some(c) = e.kind.container() {
                stroke.followers.entry(c.id()).or_default().push(e.id());
            }
        }
        stroke
    }
}

impl Editor {
    pub(crate) fn erase_to(&mut self, at: Option<ScreenPt>, mods: Modifiers) -> Redraw {
        let Some(stroke) = &mut self.eraser else {
            return Redraw::Nothing;
        };
        let Some(point) = at
            .map(|p| self.camera.screen_to_world(p))
            .or(stroke.previous)
        else {
            return Redraw::Nothing;
        };
        if !point.is_finite() {
            return Redraw::Nothing;
        }
        let previous = stroke.previous.unwrap_or(point);
        let tolerance = 8.0 / self.camera.zoom();
        let steps = ((point.x - previous.x).hypot(point.y - previous.y) / tolerance)
            .ceil()
            .clamp(1.0, 10000.0) as usize;
        stroke.candidates.clear();
        let sweep = pen_core::WorldRect::from_corners(previous, point);
        if let Some(index) = &stroke.index {
            index.query(
                pen_core::WorldRect::from_centre(
                    sweep.centre(),
                    sweep.width() / 2.0 + tolerance,
                    sweep.height() / 2.0 + tolerance,
                ),
                &mut stroke.candidates,
            );
        }
        let mut changed = false;
        for candidate in &stroke.candidates {
            let Some(e) = self.document.get(*candidate) else {
                continue;
            };
            let hit = (0..=steps).any(|i| {
                let t = i as f32 / steps as f32;
                e.hit_test(
                    WorldPt::new(
                        previous.x + (point.x - previous.x) * t,
                        previous.y + (point.y - previous.y) * t,
                    ),
                    tolerance,
                )
            });
            if !hit {
                continue;
            }
            let id = bound::owner(&self.document, e.id());
            if mods.alt {
                changed |= stroke.erased.remove(&id).is_some();
            } else if !stroke.erased.contains_key(&id)
                && let Some(owner) = self.document.get(id).filter(|e| !e.locked)
            {
                stroke.erased.insert(id, owner.clone());
                changed = true;
            }
        }
        stroke.previous = Some(point);
        if !changed {
            return Redraw::Nothing;
        }
        stroke.preview.clear();
        for e in stroke.erased.values() {
            let mut hidden = e.clone();
            hidden.props.opacity = 0;
            stroke.preview.push(hidden);
            if let Some(followers) = stroke.followers.get(&e.id()) {
                for id in followers {
                    if let Some(text) = self.document.get(*id) {
                        let mut hidden = text.clone();
                        hidden.props.opacity = 0;
                        stroke.preview.push(hidden);
                    }
                }
            }
        }
        Redraw::Draft
    }

    pub(crate) fn finish_erase(&mut self) -> Redraw {
        let Some(stroke) = self.eraser.take() else {
            return Redraw::Nothing;
        };
        if stroke.erased.is_empty() {
            return Redraw::Draft;
        }
        let commands = stroke
            .erased
            .keys()
            .map(|id| Command::Remove { id: *id })
            .collect();
        let before = self.selection.clone();
        self.selection.clear();
        if self.execute(Command::Group(commands), before) {
            Redraw::Scene
        } else {
            Redraw::Draft
        }
    }
}
