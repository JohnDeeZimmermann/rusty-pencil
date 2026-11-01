//! Binding previews are refreshed on input, never by the paint loop.
use crate::{Editor, Gesture, Modifiers};
use pen_core::{Command, Element, Kind, WorldRect, binding};

impl Editor {
    pub(crate) fn bind_arrow(&self, element: &mut Element, mods: Modifiers) {
        let Kind::Arrow(path) = &element.kind else {
            return;
        };
        if !mods.ctrl {
            element.bindings = [path.world_points().next(), path.world_points().last()].map(|p| {
                p.and_then(|p| {
                    binding::candidate(
                        &self.document,
                        pen_core::rotate(p, path.bounds().centre(), element.angle),
                        16.0 / self.camera.zoom(),
                    )
                })
            });
        } else {
            element.bindings = [None; 2];
        }
        *element = binding::fit(element, |id| self.document.get(id));
    }

    pub(crate) fn refresh_binding(&mut self) {
        self.binding_draft = None;
        self.binding_targets.clear();
        if let Some(edit) = &self.point_editor {
            for e in edit.preview() {
                for link in e.bindings.iter().flatten() {
                    if let Some(shape) = self.document.get(link.element) {
                        self.binding_targets.push(shape.bounds());
                    }
                }
            }
        }
        if let Some(Gesture::Path(draw)) = &self.gesture {
            let mods = draw.mods;
            let mut element = Element::new(self.document.draft_id(), draw.kind(true), self.style);
            self.bind_arrow(&mut element, mods);
            for link in element.bindings.iter().flatten() {
                if let Some(e) = self.document.get(link.element) {
                    self.binding_targets.push(e.bounds());
                }
            }
            self.binding_draft = Some(element.kind);
        }
    }

    pub(crate) fn wrap_text(&mut self) -> crate::Redraw {
        let Some(text) = self.selection.elements(&self.document).next().cloned() else {
            return crate::Redraw::Nothing;
        };
        let r = text.kind.bounds();
        let pad = pen_core::bound::TEXT_PADDING;
        let bounds = WorldRect::from_corners(
            pen_core::WorldPt::new(r.min().x - pad, r.min().y - pad),
            pen_core::WorldPt::new(r.max().x + pad, r.max().y + pad),
        );
        let id = self.document.alloc_id();
        let mut shape = Element::new(id, Kind::Rect(bounds), text.props);
        shape.angle = text.angle;
        shape.group_ids.clone_from(&text.group_ids);
        let mut after = text.clone();
        if let Kind::Text { container, .. } = &mut after.kind {
            *container = Some(pen_core::Container::Shape(id));
        }
        let at = self
            .document
            .elements()
            .iter()
            .position(|e| e.id() == text.id())
            .unwrap_or(0);
        let command = Command::Group(vec![
            Command::Add { element: shape, at },
            Command::Patch {
                id: text.id(),
                before: text,
                after,
            },
        ]);
        let before = self.selection.clone();
        self.selection = crate::Selection::only(id);
        if self.execute(command, before) {
            crate::Redraw::Scene
        } else {
            crate::Redraw::Nothing
        }
    }

    #[must_use]
    pub fn binding_targets(&self) -> &[WorldRect] {
        &self.binding_targets
    }
}
