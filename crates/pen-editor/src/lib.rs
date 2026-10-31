//! GUI-neutral editor state. Every gesture is driven by input events; document mutations
//! happen only on commit, while live previews contain only the elements being touched.
#![forbid(unsafe_code)]

mod camera;
mod history;
mod input;
mod selection;
mod selection_input;
mod tool;
mod tools;

use history::History;
use pen_core::{Command, Document, Element, Kind, Props, WorldRect};
use tools::{select::SelectionDrag, shape::ShapeDrag};

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
    preview: Vec<Element>,
}

/// Uncommitted shape geometry, in world points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draft {
    pub kind: Kind,
    pub props: Props,
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
        &self.preview
    }
    #[must_use]
    pub fn selection_bounds(&self) -> Option<WorldRect> {
        if self.preview.is_empty() {
            self.selection.bounds(&self.document)
        } else {
            self.preview
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
        match self.gesture {
            Some(Gesture::Drawing(drag)) => Some(Draft {
                kind: drag.kind(),
                props: self.style,
            }),
            _ => None,
        }
    }

    pub fn handle(&mut self, event: InputEvent) -> Redraw {
        match event {
            InputEvent::Pan { by } => {
                self.camera.pan_by(by);
                self.camera_moved()
            }
            InputEvent::Zoom { about, factor } => {
                self.camera.zoom_about(about, factor);
                self.camera_moved()
            }
            InputEvent::PointerDown { at, mods } => {
                self.cancel();
                let origin = self.camera.screen_to_world(at);
                if !origin.is_finite() {
                    return Redraw::Nothing;
                }
                if self.tool == Tool::Select {
                    self.selection_down(at, origin, mods);
                } else {
                    self.gesture =
                        ShapeDrag::start(self.tool, origin, at, mods).map(Gesture::Drawing);
                }
                Redraw::Draft
            }
            InputEvent::PointerMove { at, mods } => self.drag_to(Some(at), mods),
            InputEvent::Modifiers(mods) => self.drag_to(None, mods),
            InputEvent::PointerUp { at, mods } => {
                self.drag_to(Some(at), mods);
                match self.gesture.take() {
                    Some(Gesture::Drawing(drag)) => self.commit_shape(drag),
                    Some(Gesture::Selecting(drag)) => self.selection_up(drag),
                    None => Redraw::Nothing,
                }
            }
            InputEvent::Cancel => {
                if self.cancel() {
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            InputEvent::Key {
                key: Key::Escape, ..
            } => {
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
            Some(Gesture::Selecting(d)) => Some((d.pointer, d.mods)),
            None => None,
        };
        if let Some((at, mods)) = pointer {
            self.drag_to(Some(at), mods);
            Redraw::Draft
        } else {
            Redraw::Camera
        }
    }

    fn drag_to(&mut self, at: Option<ScreenPt>, mods: Modifiers) -> Redraw {
        match &mut self.gesture {
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
            Some(Gesture::Selecting(_)) => self.selection_update(at, mods),
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
        self.history
            .execute(&mut self.document, command, before, self.selection.clone())
            .is_ok()
    }

    fn cancel(&mut self) -> bool {
        let Some(gesture) = self.gesture.take() else {
            return false;
        };
        if let Gesture::Selecting(drag) = gesture {
            self.selection = drag.before;
        }
        self.preview.clear();
        true
    }

    fn select_tool(&mut self, tool: Tool) -> Redraw {
        if self.tool == tool {
            return Redraw::Nothing;
        }
        self.cancel();
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
