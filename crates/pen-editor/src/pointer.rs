//! Pointer routing between selection, point editing, path drawing and navigation.
use crate::tools::{path::PathDraw, points::PointEditor, select::SelectionDrag, shape::ShapeDrag};
use crate::{Editor, Gesture, Modifiers, Redraw, ScreenPt, Selection, Tool};
use pen_core::{Command, Element, Path};

impl Editor {
    pub(crate) fn pointer_down(&mut self, at: ScreenPt, mods: Modifiers) -> Redraw {
        if !self.camera.screen_to_world(at).is_finite() {
            return Redraw::Nothing;
        }
        if self.navigation.space || self.tool == Tool::Hand {
            self.navigation.pan = Some(at);
            return Redraw::Camera;
        }
        if let Some(edit) = &mut self.point_editor {
            if edit.press(&self.document, at, &self.camera) {
                return Redraw::Draft;
            }
            self.point_editor = None;
        }
        if let Some(Gesture::Path(draw)) = &mut self.gesture {
            draw.press(at, mods, &self.camera);
            return Redraw::Draft;
        }
        self.cancel();
        self.gesture = if self.tool == Tool::Select {
            Some(Gesture::Selecting(SelectionDrag::begin(
                &self.document,
                &mut self.selection,
                &self.camera,
                at,
                mods,
            )))
        } else if matches!(self.tool, Tool::Line | Tool::Arrow | Tool::Freedraw) {
            Some(Gesture::Path(PathDraw::begin(
                self.tool,
                at,
                &self.camera,
                mods,
            )))
        } else {
            ShapeDrag::start(self.tool, self.camera.screen_to_world(at), at, mods)
                .map(Gesture::Drawing)
        };
        Redraw::Draft
    }

    pub(crate) fn pointer_up(&mut self, at: ScreenPt, mods: Modifiers) -> Redraw {
        if self.is_panning() {
            let redraw = self.pan_to(at);
            self.navigation.pan = None;
            return redraw;
        }
        self.drag_to(Some(at), mods);
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
        if self.execute(command, self.selection.clone()) {
            Redraw::Scene
        } else {
            Redraw::Draft
        }
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
            .find(|e| e.hit_test(point, 5.0 / self.camera.zoom()));
        if let Some(element) = hit.filter(|e| e.kind.is_linear()) {
            self.selection = Selection::only(element.id());
            return self.begin_point_edit();
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
        if !element.kind.is_linear() {
            return Redraw::Nothing;
        }
        let id = element.id();
        self.cancel();
        self.tool = Tool::Select;
        self.point_editor = Some(PointEditor::new(id));
        Redraw::Tool
    }

    /// Editable vertices and the active vertex index. Geometry includes the live drag.
    #[must_use]
    pub fn editable_points(&self) -> Option<(&Path, Option<usize>)> {
        let editor = self.point_editor.as_ref()?;
        Some((editor.path(&self.document)?, editor.active))
    }
}
