//! Text drafts stay outside the document until one reversible commit.
use crate::{Editor, History, Redraw, ScreenPt, Selection, Tool};
use pen_core::{
    Command, Container, Document, Element, ElementId, Font, Kind, TextAlign, WorldPt, WorldRect,
    bound,
};

#[derive(Debug, Clone)]
pub struct TextSession {
    pub element: Element,
    before: Option<Element>,
    selection: Selection,
    /// The container as committed when the session began; growth is measured from it.
    container: Option<Element>,
    /// The container grown to fit the text so far, committed with the text.
    grown: Option<Element>,
    pub(crate) hidden: Vec<Element>,
}

impl TextSession {
    /// The element this text belongs to, with any growth from typing applied.
    #[must_use]
    pub fn container(&self) -> Option<&Element> {
        self.grown.as_ref().or(self.container.as_ref())
    }
}

/// What a new text session edits.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TextTarget {
    /// New free text with its top-left at this screen point.
    At(ScreenPt),
    Existing(ElementId),
    /// The container's text, created if it has none.
    Into(Container),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextDefaults {
    pub(crate) font: Font,
    pub(crate) size: f32,
    pub(crate) align: TextAlign,
}
impl Default for TextDefaults {
    fn default() -> Self {
        Self {
            font: Font::HandDrawn,
            size: 20.0,
            align: TextAlign::Left,
        }
    }
}

/// Places bound `text` of measured `(width, height)` on `container`, in world points. A shape
/// grows to fit and the grown container is returned; a label is re-centred at its new size.
pub(crate) fn place(text: &mut Element, container: &Element, size: (f32, f32)) -> Option<Element> {
    let (w, h) = size;
    if !w.is_finite() || !h.is_finite() {
        return None;
    }
    match text.kind.container() {
        Some(Container::Shape(_)) => {
            let grown = bound::grown_to_fit(&container.kind, h).map(|r| container.with_bounds(r));
            *text = bound::fit(text, grown.as_ref().unwrap_or(container));
            grown
        }
        Some(Container::Path(_)) => {
            if let Kind::Text { bounds, .. } = &mut text.kind {
                let p = bounds.min();
                *bounds = WorldRect::from_corners(p, WorldPt::new(p.x + w.max(1.0), p.y + h));
            }
            *text = bound::fit(text, container);
            None
        }
        None => None,
    }
}

impl Editor {
    #[must_use]
    pub fn text_session(&self) -> Option<&TextSession> {
        self.text_session.as_ref()
    }

    /// Replaces the scene at a file boundary; old history and drafts cannot reach into it.
    pub fn replace_document(
        &mut self,
        document: Document,
        camera: crate::Camera,
        style: pen_core::Props,
    ) {
        self.document = document;
        self.camera = camera;
        self.style = style;
        self.selection.clear();
        self.history = History::default();
        self.property_edit = None;
        self.gesture = None;
        self.point_editor = None;
        self.text_session = None;
        self.navigation.refresh(&self.document);
        self.refresh_followers();
    }

    pub(crate) fn begin_text(&mut self, target: TextTarget) -> Redraw {
        self.finish_text();
        self.cancel();
        self.point_editor = None;
        let target = match target {
            TextTarget::Into(link) => bound::bound_text(&self.document, link.id())
                .map_or(target, |text| TextTarget::Existing(text.id())),
            other => other,
        };
        let d = self.text_defaults;
        let new_text = |id, p: WorldPt, align, container| {
            Element::new(
                id,
                Kind::Text {
                    content: String::new(),
                    font: d.font,
                    size: d.size,
                    align,
                    bounds: WorldRect::from_corners(
                        p,
                        WorldPt::new(p.x + 1.0, p.y + d.size * 1.25),
                    ),
                    container,
                },
                self.style,
            )
        };
        let (before, element) = match target {
            TextTarget::At(at) => {
                let p = self.camera.screen_to_world(at);
                let id = self.document.alloc_id();
                (None, new_text(id, p, d.align, None))
            }
            TextTarget::Existing(id) => {
                let Some(e) = self.document.get(id) else {
                    return Redraw::Nothing;
                };
                (Some(e.clone()), e.clone())
            }
            TextTarget::Into(link) => {
                let id = self.document.alloc_id();
                (
                    None,
                    new_text(id, WorldPt::ORIGIN, TextAlign::Centre, Some(link)),
                )
            }
        };
        let container = element
            .kind
            .container()
            .and_then(|c| self.document.get(c.id()))
            .cloned();
        let mut element = element;
        if let Some(container) = &container {
            element = bound::fit(&element, container);
        }
        let selection = self.selection.clone();
        self.selection.clear();
        let mut session = TextSession {
            element,
            before,
            selection,
            container,
            grown: None,
            hidden: Vec::new(),
        };
        session.refresh_hidden();
        self.text_session = Some(session);
        Redraw::Tool
    }

    /// `measured` is the text's laid-out size; free text also takes its position from it.
    pub(crate) fn update_text(&mut self, value: String, measured: WorldRect) -> Redraw {
        let Some(session) = &mut self.text_session else {
            return Redraw::Nothing;
        };
        if let Kind::Text { content, .. } = &mut session.element.kind {
            *content = value;
        }
        session.resize(measured);
        Redraw::Draft
    }

    pub(crate) fn finish_text(&mut self) -> Redraw {
        let Some(session) = self.text_session.take() else {
            return Redraw::Nothing;
        };
        let Kind::Text { content, .. } = &session.element.kind else {
            return Redraw::Nothing;
        };
        let empty = content.trim().is_empty();
        let owner = session.container.as_ref().map(Element::id);
        if !self.settings.tool_lock {
            self.tool = Tool::Select;
        }
        // Bound text hands selection back to its container, even when left empty.
        self.selection = match owner {
            Some(owner) => Selection::only(owner),
            None if empty => Selection::default(),
            None => Selection::only(session.element.id()),
        };
        let mut commands = Vec::new();
        match &session.before {
            Some(before) if empty => commands.push(Command::Remove { id: before.id() }),
            Some(before)
                if before.kind == session.element.kind && before.props == session.element.props => {
            }
            Some(before) => commands.push(Command::Patch {
                id: before.id(),
                before: before.clone(),
                after: session.element.clone(),
            }),
            None if empty => {}
            None => {
                // Bound text sits just above its container, so nothing can come between.
                let at = owner
                    .and_then(|id| self.document.elements().iter().position(|e| e.id() == id))
                    .map_or(self.document.elements().len(), |i| i + 1);
                commands.push(Command::Add {
                    element: session.element.clone(),
                    at,
                });
            }
        }
        if !empty && let (Some(before), Some(after)) = (&session.container, &session.grown) {
            commands.push(Command::Patch {
                id: before.id(),
                before: before.clone(),
                after: after.clone(),
            });
        }
        if commands.is_empty() {
            return Redraw::Tool;
        }
        self.execute(Command::Group(commands), session.selection);
        Redraw::Scene
    }
}

impl TextSession {
    /// Applies a newly measured size: free text takes the rect as is, bound text is placed on
    /// its container (which may grow).
    pub(crate) fn resize(&mut self, measured: WorldRect) {
        if !measured.min().is_finite() || !measured.max().is_finite() {
            return;
        }
        if let Some(container) = &self.container {
            let size = (measured.width(), measured.height());
            self.grown = place(&mut self.element, container, size);
        } else if let Kind::Text { bounds, .. } = &mut self.element.kind {
            *bounds = measured;
        }
        self.refresh_hidden();
    }

    /// The committed text is hidden under the editor; a grown container replaces its paint.
    fn refresh_hidden(&mut self) {
        self.hidden.clear();
        if let Some(old) = &self.before {
            let mut e = old.clone();
            if let Kind::Text { content, .. } = &mut e.kind {
                content.clear();
            }
            self.hidden.push(e);
        }
        if let Some(grown) = &self.grown {
            self.hidden.push(grown.clone());
        }
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
