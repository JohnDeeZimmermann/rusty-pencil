//! GUI-neutral editor state. Every gesture is driven by input events; document mutations
//! happen only on commit, while live previews contain only the elements being touched.
#![forbid(unsafe_code)]

mod actions;
mod arrange;
mod arrange_positions;
mod binding;
mod images;
pub use actions::Action;
mod camera;
mod eraser;
mod eraser_index;
mod follow;
mod history;
mod input;
mod navigation;
mod pointer;
mod properties;
pub use properties::GRID_SPACING;
mod selection;
mod shortcuts;
mod style_clipboard;
mod text;
mod text_style;
mod tool;
pub use text::TextSession;
mod tools;

use history::History;
use navigation::Navigation;
use pen_core::{Command, Document, Element, Kind, Props, WorldRect};
use tools::{path::PathDraw, points::PointEditor, select::SelectionDrag, shape::ShapeDrag};

pub use camera::{Camera, ScreenPt, ScreenVec};
pub use input::{InputEvent, Key, Modifiers};
pub use pen_core::file::AppState;
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
    binding_draft: Option<Kind>,
    binding_targets: Vec<WorldRect>,
    eraser: Option<eraser::Eraser>,
    group_scope: Option<pen_core::GroupId>,
    clipboard: Vec<Element>,
    copied_style: Option<arrange::CopiedStyle>,
    camera: Camera,
    document: Document,
    tool: Tool,
    style: Props,
    settings: pen_core::Settings,
    property_edit: Option<properties::PropertyEdit>,
    gesture: Option<Gesture>,
    selection: Selection,
    history: History,
    navigation: Navigation,
    point_editor: Option<PointEditor>,
    text_session: Option<TextSession>,
    text_defaults: text::TextDefaults,
    followers: follow::Followers,
    text_focus: text_style::TextFocus,
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
    #[must_use]
    pub fn selection_locked(&self) -> bool {
        self.selection.elements(&self.document).any(|e| e.locked)
    }

    /// Only touched elements, replacing their committed paint during a transform. Includes
    /// bound text following a previewed container.
    #[must_use]
    pub fn preview(&self) -> &[Element] {
        self.followers
            .composed()
            .unwrap_or_else(|| self.base_preview())
    }

    fn base_preview(&self) -> &[Element] {
        if let Some(stroke) = &self.eraser {
            return &stroke.preview;
        }
        if let Some(edit) = &self.property_edit
            && edit.text_is_absent()
        {
            return &edit.elements;
        }
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
        // Writing inside a container frames nothing: the container itself is the frame.
        if self
            .text_session
            .as_ref()
            .is_some_and(|s| s.element.kind.container().is_some())
        {
            return None;
        }
        let preview = self.base_preview();
        if preview.is_empty() {
            self.selection.bounds(&self.document)
        } else {
            preview.iter().map(Element::bounds).reduce(WorldRect::union)
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
                kind: self
                    .binding_draft
                    .as_ref()
                    .map_or_else(|| drag.kind(true), Clone::clone),
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
        let redraw = self.dispatch(event);
        self.refresh_binding();
        self.refresh_followers();
        redraw
    }

    /// Call after any change to the document or the preview.
    fn refresh_followers(&mut self) {
        let mut followers = std::mem::take(&mut self.followers);
        followers.update(&self.document, self.base_preview());
        self.followers = followers;
        self.text_focus.update(&self.document, &self.selection);
    }

    fn dispatch(&mut self, event: InputEvent) -> Redraw {
        match event {
            InputEvent::InsertImage { file, at } => self.insert_image(file, at),
            InputEvent::CropImage(crop) => self.crop_image(crop),
            InputEvent::Context { at } => {
                self.finish_text();
                self.cancel();
                self.tool = Tool::Select;
                let point = self.camera.screen_to_world(at);
                if let Some(e) = self
                    .document
                    .elements()
                    .iter()
                    .rev()
                    .find(|e| e.hit_test(point, 5.0 / self.camera.zoom()))
                {
                    let id = pen_core::bound::owner(&self.document, e.id());
                    if !self.selection.contains(id) {
                        self.selection = Selection::only(id);
                        self.selection
                            .expand_group(&self.document, id, self.group_scope);
                    }
                }
                Redraw::Tool
            }
            InputEvent::Action(action) => self.action(action),
            InputEvent::PasteScene(scene) => {
                if scene.document().is_err() {
                    return Redraw::Nothing;
                }
                self.paste_with_files(
                    &scene.elements,
                    pen_core::WorldPt::new(20.0, 20.0),
                    &scene.files,
                )
            }
            InputEvent::Paste(elements) => {
                self.paste_elements(&elements, pen_core::WorldPt::new(20.0, 20.0))
            }
            InputEvent::TextChanged { content, bounds } => self.update_text(content, bounds),
            InputEvent::FinishText => self.finish_text(),
            InputEvent::TextStyle { font, size, align } => self.set_text_style(font, size, align),
            InputEvent::BeginProperty => self.begin_property(),
            InputEvent::EndProperty { cancel } => self.end_property(cancel),
            InputEvent::Property(p) => self.set_property(p),
            InputEvent::Settings(settings) => {
                self.settings = settings;
                Redraw::Scene
            }
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
                    if self
                        .point_editor
                        .as_ref()
                        .is_some_and(|edit| edit.endpoint_only)
                    {
                        self.point_editor = None;
                    }
                    return Redraw::Draft;
                }
                if self.point_editor.take().is_some() {
                    return Redraw::Tool;
                }
                if self.cancel() {
                    Redraw::Draft
                } else if !self.selection.is_empty() {
                    self.group_scope = None;
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
            .is_some_and(|e| e.camera_moved(&self.camera, self.settings.grid, &self.document))
        {
            Redraw::Draft
        } else {
            Redraw::Camera
        }
    }

    fn drag_to(&mut self, at: Option<ScreenPt>, mods: Modifiers) -> Redraw {
        if self.eraser.is_some() {
            return self.erase_to(at, mods);
        }
        if let Some(edit) = &mut self.point_editor {
            return if edit.update(at, mods, &self.camera, self.settings.grid, &self.document) {
                Redraw::Draft
            } else {
                Redraw::Nothing
            };
        }
        match &mut self.gesture {
            Some(Gesture::Path(drag)) => {
                if drag.update(at, mods, &self.camera, self.settings.grid) {
                    Redraw::Draft
                } else {
                    Redraw::Nothing
                }
            }
            Some(Gesture::Drawing(drag)) => {
                let before = *drag;
                if let Some(at) = at {
                    let point =
                        properties::snap(self.camera.screen_to_world(at), self.settings.grid);
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
                if drag.update(
                    at,
                    mods,
                    &self.camera,
                    self.settings.grid,
                    &mut self.document,
                ) {
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
        self.commit_drawing(command, id)
    }

    /// The only way edits reach the document. Bound text follows inside the same history
    /// entry, so one undo restores a container and its text together.
    fn execute(&mut self, command: Command, before: Selection) -> bool {
        let command = pen_core::binding::follow(&self.document, command);
        let command = pen_core::bound::follow(&self.document, command);
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
        let erased = self.eraser.take().is_some();
        let property = self.property_edit.is_some();
        if property {
            self.end_property(true);
        }
        let panning = self.navigation.pan.take().is_some() || property;
        let editing = self.point_editor.as_mut().is_some_and(PointEditor::cancel);
        let Some(gesture) = self.gesture.take() else {
            return panning || editing || erased;
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

#[cfg(test)]
mod bound_text_tests;

#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod arrange_tests;

#[cfg(test)]
mod binding_tests;

#[cfg(test)]
mod image_tests;
