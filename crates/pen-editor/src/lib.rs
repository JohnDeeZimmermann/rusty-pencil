//! GUI-neutral editor state. Every gesture is driven by input events; document mutations
//! happen only on commit, while live previews contain only the elements being touched.
#![forbid(unsafe_code)]

mod camera;
mod history;
mod input;
mod navigation;
mod pointer;
mod selection;
mod shortcuts;
mod text;
mod tool;
pub use text::TextSession;
mod tools;

use history::History;
use navigation::Navigation;
use pen_core::{Command, Document, Element, Kind, Props, WorldRect};
use tools::{path::PathDraw, points::PointEditor, select::SelectionDrag, shape::ShapeDrag};

pub use camera::{Camera, ScreenPt, ScreenVec};
pub use input::{InputEvent, Key, Modifiers};
pub use selection::{HANDLE_SIZE, Handle, SELECTION_PADDING, Selection};
pub use tool::Tool;

/// Screen points of movement required before a press becomes a drag.
pub const CLICK_SLOP: f32 = 3.0;

#[derive(Debug, Clone)]
enum Gesture {
    Drawing(ShapeDrag),
    Selecting(SelectionDrag),
    Path(PathDraw),
}

/// Owned by the app; no window, renderer, or shared mutable state.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    camera: Camera,
    document: Document,
    tool: Tool,
    style: Props,
    gesture: Option<Gesture>,
    selection: Selection,
    history: History,
    navigation: Navigation,
    point_editor: Option<PointEditor>,
    text_session: Option<TextSession>,
    text_defaults: text::TextDefaults,
}

/// Uncommitted shape geometry, in world points.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub kind: Kind,
    pub props: Props,
    pub seed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redraw {
    Nothing,
    Camera,
    /// Only live preview / selection feedback changed.
    Draft,
    Scene,
    Tool,
}

impl Editor {
    #[must_use]
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            ..Self::default()
        }
    }
    #[must_use]
    pub fn camera(&self) -> &Camera {
        &self.camera
    }
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }
    #[must_use]
    pub fn tool(&self) -> Tool {
        self.tool
    }
    #[must_use]
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
    /// Only touched elements, replacing their committed paint during a transform.
    #[must_use]
    pub fn preview(&self) -> &[Element] {
        if let Some(edit) = &self.text_session {
            return &edit.hidden;
        }
        if let Some(edit) = &self.point_editor {
            return edit.preview();
        }
        match &self.gesture {
            Some(Gesture::Selecting(drag)) => drag.preview(),
            _ => &[],
        }
    }
    #[must_use]
    pub fn selection_bounds(&self) -> Option<WorldRect> {
        let preview = self.preview();
        if preview.is_empty() {
            self.selection.bounds(&self.document)
        } else {
            preview
                .iter()
                .map(|e| e.kind.bounds())
                .reduce(WorldRect::union)
        }
    }
    #[must_use]
    pub fn marquee(&self) -> Option<WorldRect> {
        match &self.gesture {
            Some(Gesture::Selecting(drag)) => drag.marquee(),
            _ => None,
        }
    }
    #[must_use]
    pub fn draft(&self) -> Option<Draft> {
        match &self.gesture {
            Some(Gesture::Path(drag)) => Some(Draft {
                kind: drag.kind(true),
                props: self.style,
                seed: self.document.draft_id().seed(),
            }),
            Some(Gesture::Drawing(drag)) => Some(Draft {
                kind: drag.kind(),
                props: self.style,
                seed: self.document.draft_id().seed(),
            }),
            _ => None,
        }
    }

    pub fn handle(&mut self, event: InputEvent) -> Redraw {
        match event {
            InputEvent::TextChanged { content, bounds } => self.update_text(content, bounds),
            InputEvent::FinishText => self.finish_text(),
            InputEvent::TextStyle { font, size, align } => self.set_text_style(font, size, align),
            InputEvent::Roughness(roughness) => self.set_roughness(roughness),
            InputEvent::Pan { by } => {
                self.camera.pan_by(by);
                self.camera_moved()
            }
            InputEvent::Zoom { about, factor } => {
                self.camera.zoom_about(about, factor);
                self.camera_moved()
            }
            InputEvent::Viewport(size) => {
                if size.dx.is_finite() && size.dy.is_finite() && size.dx > 0.0 && size.dy > 0.0 {
                    self.navigation.viewport = size;
                }
                Redraw::Camera
            }
            InputEvent::FitContent => self.fit(false),
            InputEvent::Space(down) => {
                if self.navigation.space == down {
                    return Redraw::Nothing;
                }
                self.navigation.space = down;
                Redraw::Tool
            }
            InputEvent::PanStart { at } => {
                if at.x.is_finite() && at.y.is_finite() {
                    self.navigation.pan = Some(at);
                }
                Redraw::Camera
            }
            InputEvent::PanMove { at } => self.pan_to(at),
            InputEvent::PanEnd => {
                self.navigation.pan = None;
                Redraw::Camera
            }
            InputEvent::PointerDown { at, mods } => self.pointer_down(at, mods),
            InputEvent::PointerMove { at, mods } => {
                if self.is_panning() {
                    self.pan_to(at)
                } else {
                    self.drag_to(Some(at), mods)
                }
            }
            InputEvent::Modifiers(mods) => self.drag_to(None, mods),
            InputEvent::PointerUp { at, mods } => self.pointer_up(at, mods),
            InputEvent::DoubleClick { at, mods } => self.double_click(at, mods),
            InputEvent::Cancel => {
                let space = std::mem::take(&mut self.navigation.space);
                if space {
                    self.cancel();
                    return Redraw::Tool;
                }
                if self.cancel() {
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            InputEvent::Key {
                key: Key::Escape, ..
            } => {
                if self.text_session.is_some() {
                    return self.finish_text();
                }
                if self.tool != Tool::Freedraw && matches!(self.gesture, Some(Gesture::Path(_))) {
                    return self.finish_path(false);
                }
                if self.point_editor.as_mut().is_some_and(PointEditor::cancel) {
                    return Redraw::Draft;
                }
                if self.point_editor.take().is_some() {
                    return Redraw::Tool;
                }
                if self.cancel() {
                    Redraw::Draft
                } else if !self.selection.is_empty() {
                    self.selection.clear();
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            InputEvent::Key { key, mods } => self.key(key, mods),
            InputEvent::SelectTool(tool) => self.select_tool(tool),
        }
    }

    fn camera_moved(&mut self) -> Redraw {
        let pointer = match &self.gesture {
            Some(Gesture::Drawing(d)) => Some((d.pointer, d.mods)),
            Some(Gesture::Selecting(d)) => Some(d.pointer()),
            Some(Gesture::Path(d)) => Some((d.pointer, d.mods)),
            None => None,
        };
        if let Some((at, mods)) = pointer {
            self.drag_to(Some(at), mods);
            Redraw::Draft
        } else if self
            .point_editor
            .as_mut()
            .is_some_and(|e| e.camera_moved(&self.camera))
        {
            Redraw::Draft
        } else {
            Redraw::Camera
        }
    }

    fn drag_to(&mut self, at: Option<ScreenPt>, mods: Modifiers) -> Redraw {
        if let Some(edit) = &mut self.point_editor {
            return if edit.update(at, mods, &self.camera) {
                Redraw::Draft
            } else {
                Redraw::Nothing
            };
        }
        match &mut self.gesture {
            Some(Gesture::Path(drag)) => {
                if drag.update(at, mods, &self.camera) {
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            Some(Gesture::Drawing(drag)) => {
                let before = *drag;
                if let Some(at) = at {
                    let point = self.camera.screen_to_world(at);
                    if point.is_finite() {
                        drag.current = point;
                        drag.pointer = at;
                    }
                }
                drag.mods = mods;
                if *drag == before {
                    Redraw::Nothing
                } else {
                    Redraw::Draft
                }
            }
            Some(Gesture::Selecting(drag)) => {
                if drag.update(at, mods, &self.camera) {
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            None => Redraw::Nothing,
        }
    }

    fn commit_shape(&mut self, drag: ShapeDrag) -> Redraw {
        let bounds = drag.bounds();
        let span = bounds.width().max(bounds.height()) * self.camera.zoom();
        if bounds.is_degenerate() || span < CLICK_SLOP {
            return Redraw::Draft;
        }
        let id = self.document.alloc_id();
        let command = Command::Add {
            element: Element::new(id, drag.kind(), self.style),
            at: self.document.elements().len(),
        };
        if self.execute(command, self.selection.clone()) {
            Redraw::Scene
        } else {
            Redraw::Draft
        }
    }

    fn execute(&mut self, command: Command, before: Selection) -> bool {
        let success = self
            .history
            .execute(&mut self.document, command, before, self.selection.clone())
            .is_ok();
        if success {
            self.navigation.refresh(&self.document);
        }
        success
    }

    fn cancel(&mut self) -> bool {
        let panning = self.navigation.pan.take().is_some();
        let editing = self.point_editor.as_mut().is_some_and(PointEditor::cancel);
        let Some(gesture) = self.gesture.take() else {
            return panning || editing;
        };
        if let Gesture::Selecting(drag) = gesture {
            drag.cancel(&mut self.selection);
        }
        true
    }

    fn select_tool(&mut self, tool: Tool) -> Redraw {
        if self.tool == tool && self.text_session.is_none() {
            return Redraw::Nothing;
        }
        self.finish_text();
        self.cancel();
        self.point_editor = None;
        self.tool = tool;
        if tool != Tool::Select {
            self.selection.clear();
        }
        Redraw::Tool
    }
}

#[cfg(test)]
mod camera_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod shape_tests;

#[cfg(test)]
mod path_tests;

#[cfg(test)]
mod navigation_tests;
