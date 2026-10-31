//! Interaction over a `pen-core` document: tools, selection, camera and history.
//!
//! Driven only by [`InputEvent`]s, so every behaviour is testable without a window. The app
//! crate translates GUI events into `InputEvent`s and presents the resulting state.
#![forbid(unsafe_code)]

mod camera;
mod input;
mod tool;
mod tools;

use pen_core::{Command, Document, Element, Kind, Props};

pub use camera::{Camera, ScreenPt, ScreenVec};
pub use input::{InputEvent, Key, Modifiers};
pub use tool::Tool;
use tools::shape::ShapeDrag;

/// A drag shorter than this many screen points is a click, and a click draws nothing.
pub const CLICK_SLOP: f32 = 3.0;

/// All editor state. The app owns exactly one and hands it `&mut` per event.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    camera: Camera,
    document: Document,
    tool: Tool,
    /// The props new elements get.
    style: Props,
    gesture: Option<ShapeDrag>,
}

/// An element being drawn: not in the document yet, rendered on the live layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Draft {
    pub kind: Kind,
    pub props: Props,
}

/// What changed after an event, so the app redraws as little as possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redraw {
    /// Nothing visible changed.
    Nothing,
    /// Only the camera moved: present the same scene with a new transform.
    Camera,
    /// The draft changed (the camera may have too); the document did not.
    Draft,
    /// The document changed (and the draft is gone).
    Scene,
    /// The active tool changed.
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

    /// The element being drawn right now, if any.
    #[must_use]
    pub fn draft(&self) -> Option<Draft> {
        self.gesture.map(|g| Draft {
            kind: g.kind(),
            props: self.style,
        })
    }

    /// Applies one input event. Returns what kind of redraw it needs.
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
                let origin = self.camera.screen_to_world(at);
                self.gesture = origin.is_finite().then_some(ShapeDrag {
                    tool: self.tool,
                    origin,
                    current: origin,
                    pointer: at,
                    mods,
                });
                Redraw::Draft
            }
            InputEvent::PointerMove { at, mods } => self.drag_to(Some(at), mods),
            InputEvent::Modifiers(mods) => self.drag_to(None, mods),
            InputEvent::PointerUp { at, mods } => {
                self.drag_to(Some(at), mods);
                self.commit()
            }
            InputEvent::Cancel
            | InputEvent::Key {
                key: Key::Escape, ..
            } => {
                if self.gesture.take().is_some() {
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            InputEvent::Key { key, mods } => match Tool::for_key(key, mods) {
                Some(tool) => self.select_tool(tool),
                None => Redraw::Nothing,
            },
            InputEvent::SelectTool(tool) => self.select_tool(tool),
        }
    }

    /// The pointer stays put on screen while the camera moves, so a drag in progress follows
    /// it to the new world point under the pointer.
    fn camera_moved(&mut self) -> Redraw {
        match self.gesture {
            Some(drag) => {
                self.drag_to(Some(drag.pointer), drag.mods);
                Redraw::Draft
            }
            None => Redraw::Camera,
        }
    }

    fn drag_to(&mut self, at: Option<ScreenPt>, mods: Modifiers) -> Redraw {
        let Some(drag) = &mut self.gesture else {
            return Redraw::Nothing;
        };
        let before = *drag;
        if let Some(at) = at {
            let p = self.camera.screen_to_world(at);
            if p.is_finite() {
                drag.current = p;
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

    fn commit(&mut self) -> Redraw {
        let Some(drag) = self.gesture.take() else {
            return Redraw::Nothing;
        };
        let bounds = drag.bounds();
        let zoom = self.camera.zoom();
        let span = bounds.width().max(bounds.height()) * zoom;
        if bounds.is_degenerate() || span < CLICK_SLOP {
            return Redraw::Draft;
        }
        let id = self.document.alloc_id();
        let element = Element::new(id, drag.kind(), self.style);
        let at = self.document.elements().len();
        match self.document.apply(Command::Add { element, at }) {
            // The inverse goes to history in M2.
            Ok(_inverse) => Redraw::Scene,
            Err(_) => Redraw::Draft,
        }
    }

    fn select_tool(&mut self, tool: Tool) -> Redraw {
        if self.tool == tool {
            return Redraw::Nothing;
        }
        self.tool = tool;
        Redraw::Tool
    }
}

#[cfg(test)]
mod camera_tests;
#[cfg(test)]
mod shape_tests;
