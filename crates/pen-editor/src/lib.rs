//! Interaction over a `pen-core` document: tools, selection, camera and history.
//!
//! Driven only by [`InputEvent`]s, so every behaviour is testable without a window. The app
//! crate translates GUI events into `InputEvent`s and presents the resulting state.
#![forbid(unsafe_code)]

mod camera;
mod input;

pub use camera::{Camera, ScreenPt, ScreenVec};
pub use input::InputEvent;

/// All editor state. The app owns exactly one and hands it `&mut` per event.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    camera: Camera,
}

impl Editor {
    #[must_use]
    pub fn new(camera: Camera) -> Self {
        Self { camera }
    }

    #[must_use]
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Applies one input event. Returns what kind of redraw it needs.
    pub fn handle(&mut self, event: InputEvent) -> Redraw {
        match event {
            InputEvent::Pan { by } => {
                self.camera.pan_by(by);
                Redraw::Camera
            }
            InputEvent::Zoom { about, factor } => {
                self.camera.zoom_about(about, factor);
                Redraw::Camera
            }
        }
    }
}

/// What changed after an event, so the app redraws as little as possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redraw {
    /// Only the camera moved: present the same scene with a new transform.
    Camera,
}

#[cfg(test)]
mod camera_tests;
