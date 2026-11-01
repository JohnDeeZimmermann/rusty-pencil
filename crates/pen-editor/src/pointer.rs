//! Pointer routing between selection, point editing, path drawing and navigation.
use crate::text::TextTarget;
use crate::tools::{path::PathDraw, points::PointEditor, select::SelectionDrag, shape::ShapeDrag};
use crate::{Editor, Gesture, Modifiers, Redraw, ScreenPt, Selection, Tool};
use pen_core::{Command, Container, Element, Kind, Path, bound};

impl Editor {
    pub(crate) fn pointer_down(&mut self, at: ScreenPt, mods: Modifiers) -> Redraw {
        if !self.camera.screen_to_world(at).is_finite() {
            return Redraw::Nothing;
        }
        if self.navigation.space || self.tool == Tool::Hand {
            self.navigation.pan = Some(at);
            return Redraw::Camera;
        }
        let finished_text = self.text_session.is_some();
        self.finish_text();
        if self.tool == Tool::Eraser {
            self.cancel();
            self.eraser = Some(crate::eraser::Eraser::new(&self.document));
            return self.erase_to(Some(at), mods);
        }
        if self.tool == Tool::Text {
            return self.begin_text(TextTarget::At(at));
        }
        if let Some(edit) = &mut self.point_editor {
            if edit.press(&self.document, at, &self.camera) {
                return Redraw::Draft;
            }
            self.point_editor = None;
        }
        if let Some(Gesture::Path(draw)) = &mut self.gesture {
            draw.press(at, mods, &self.camera, self.settings.grid);
            return Redraw::Draft;
        }
        self.cancel();
        self.gesture = if self.tool == Tool::Select {
            Some(Gesture::Selecting(SelectionDrag::begin(
                &mut self.document,
                &mut self.selection,
                &self.camera,
                at,
                mods,
                self.group_scope,
            )))
        } else if matches!(self.tool, Tool::Line | Tool::Arrow | Tool::Freedraw) {
            Some(Gesture::Path(PathDraw::begin(
                self.tool,
                at,
                &self.camera,
                mods,
                self.settings.grid,
            )))
        } else {
            ShapeDrag::start(
                self.tool,
                crate::properties::snap(self.camera.screen_to_world(at), self.settings.grid),
                at,
                mods,
            )
            .map(Gesture::Drawing)
        };
        if finished_text {
            Redraw::Scene
        } else {
            Redraw::Draft
        }
    }

    pub(crate) fn pointer_up(&mut self, at: ScreenPt, mods: Modifiers) -> Redraw {
        if self.is_panning() {
            let redraw = self.pan_to(at);
            self.navigation.pan = None;
            return redraw;
        }
        self.drag_to(Some(at), mods);
        if self.eraser.is_some() {
            return self.finish_erase();
        }
        if let Some(edit) = &mut self.point_editor {
            let command = edit.finish();
            return if command.is_some_and(|c| self.execute(c, self.selection.clone())) {
                Redraw::Scene
            } else {
                Redraw::Draft
            };
        }
        if let Some(Gesture::Path(draw)) = &mut self.gesture {
            return if draw.release() {
                self.finish_path(false)
            } else {
                Redraw::Draft
            };
        }
        match self.gesture.take() {
            Some(Gesture::Drawing(draw)) => self.commit_shape(draw),
            Some(Gesture::Selecting(drag)) => {
                if let Some((command, before)) = drag.finish(&self.document, &mut self.selection)
                    && self.execute(command, before)
                {
                    return Redraw::Scene;
                }
                Redraw::Draft
            }
            _ => Redraw::Nothing,
        }
    }

    pub(crate) fn finish_path(&mut self, include_tail: bool) -> Redraw {
        if !matches!(self.gesture, Some(Gesture::Path(_))) {
            return Redraw::Nothing;
        }
        let Some(Gesture::Path(draw)) = self.gesture.take() else {
            return Redraw::Nothing;
        };
        let Some(kind) = draw.finish(self.camera.zoom(), include_tail) else {
            return Redraw::Draft;
        };
        let id = self.document.alloc_id();
        let command = Command::Add {
            element: Element::new(id, kind, self.style),
            at: self.document.elements().len(),
        };
        self.commit_drawing(command, id)
    }

    pub(crate) fn double_click(&mut self, at: ScreenPt, mods: Modifiers) -> Redraw {
        if self.navigation.space || self.tool == Tool::Hand {
            return self.pointer_down(at, mods);
        }
        if matches!(self.gesture, Some(Gesture::Path(_))) {
            return self.finish_path(false);
        }
        if self.tool != Tool::Select {
            return self.pointer_down(at, mods);
        }
        self.cancel();
        let point = self.camera.screen_to_world(at);
        let hit = self
            .document
            .elements()
            .iter()
            .rev()
            .find(|e| {
                !e.locked
                    && !self
                        .document
                        .get(bound::owner(&self.document, e.id()))
                        .is_some_and(|owner| owner.locked)
                    && e.hit_test_interior(point, 5.0 / self.camera.zoom())
            })
            .map(|e| bound::owner(&self.document, e.id()))
            .and_then(|id| self.document.get(id));
        let Some(element) = hit else {
            // Like Excalidraw: double-click on empty canvas starts writing there.
            return self.begin_text(TextTarget::At(at));
        };
        let next_group = match self.group_scope {
            Some(scope) => element
                .group_ids
                .iter()
                .position(|g| *g == scope)
                .and_then(|i| i.checked_sub(1))
                .map(|i| element.group_ids[i]),
            None => element.group_ids.last().copied(),
        };
        if let Some(group) = next_group {
            self.group_scope = Some(group);
            self.selection = Selection::only(element.id());
            return Redraw::Draft;
        }
        if matches!(element.kind, Kind::Text { .. }) {
            return self.begin_text(TextTarget::Existing(element.id()));
        }
        // Ctrl keeps the M3 shortcut for editing a line's points.
        if mods.ctrl && element.kind.is_linear() {
            self.selection = Selection::only(element.id());
            return self.begin_point_edit();
        }
        if let Some(container) = Container::for_element(element) {
            return self.begin_text(TextTarget::Into(container));
        }
        Redraw::Draft
    }

    pub(crate) fn begin_point_edit(&mut self) -> Redraw {
        if self.selection.len() != 1 {
            return Redraw::Nothing;
        }
        let Some(element) = self.selection.elements(&self.document).next() else {
            return Redraw::Nothing;
        };
        if element.locked || !element.kind.is_linear() {
            return Redraw::Nothing;
        }
        let id = element.id();
        self.cancel();
        self.tool = Tool::Select;
        self.point_editor = Some(PointEditor::new(id));
        Redraw::Tool
    }

    /// Rotation applied to point-editing geometry, in world points and clockwise radians.
    #[must_use]
    pub fn point_edit_transform(&self) -> Option<(pen_core::WorldPt, f32)> {
        let edit = self.point_editor.as_ref()?;
        let e = edit
            .preview()
            .first()
            .or_else(|| self.document.get(edit.id))?;
        Some((e.kind.bounds().centre(), e.angle))
    }

    /// Editable vertices and the active vertex index. Geometry includes the live drag.
    #[must_use]
    pub fn editable_points(&self) -> Option<(&Path, Option<usize>)> {
        let editor = self.point_editor.as_ref()?;
        Some((editor.path(&self.document)?, editor.active))
    }
}
