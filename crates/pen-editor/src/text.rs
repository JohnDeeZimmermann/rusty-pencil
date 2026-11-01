//! Text drafts stay outside the document until one reversible commit.
use crate::{Editor, History, Redraw, ScreenPt, Selection, Tool};
use pen_core::{
    Command, Document, Element, ElementId, Font, Kind, Roughness, TextAlign, WorldPt, WorldRect,
};

#[derive(Debug, Clone)]
pub struct TextSession {
    pub element: Element,
    before: Option<Element>,
    selection: Selection,
    pub(crate) hidden: Vec<Element>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TextDefaults {
    font: Font,
    size: f32,
    align: TextAlign,
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

impl Editor {
    #[must_use]
    pub fn text_defaults(&self) -> (Font, f32, TextAlign) {
        (
            self.text_defaults.font,
            self.text_defaults.size,
            self.text_defaults.align,
        )
    }

    /// Restores validated defaults at a scene-file boundary without editing existing text.
    pub fn restore_text_defaults(&mut self, font: Font, size: f32, align: TextAlign) {
        self.text_defaults = TextDefaults { font, size, align };
    }

    #[must_use]
    pub fn text_session(&self) -> Option<&TextSession> {
        self.text_session.as_ref()
    }

    #[must_use]
    pub fn text_style(&self) -> (Font, f32, TextAlign) {
        let element = self.text_session.as_ref().map(|s| &s.element).or_else(|| {
            self.selection
                .elements(&self.document)
                .find(|e| matches!(e.kind, Kind::Text { .. }))
        });
        if let Some(Element {
            kind: Kind::Text {
                font, size, align, ..
            },
            ..
        }) = element
        {
            (*font, *size, *align)
        } else {
            (
                self.text_defaults.font,
                self.text_defaults.size,
                self.text_defaults.align,
            )
        }
    }

    #[must_use]
    pub fn roughness(&self) -> Roughness {
        self.selection
            .elements(&self.document)
            .next()
            .map_or(self.style.roughness, |e| e.props.roughness)
    }

    #[must_use]
    pub fn style(&self) -> pen_core::Props {
        self.style
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
        self.gesture = None;
        self.point_editor = None;
        self.text_session = None;
        self.navigation.refresh(&self.document);
    }

    pub(crate) fn begin_text(&mut self, at: ScreenPt, id: Option<ElementId>) -> Redraw {
        self.finish_text();
        self.cancel();
        self.point_editor = None;
        let before = id.and_then(|id| self.document.get(id)).cloned();
        let element = before.clone().unwrap_or_else(|| {
            let p = self.camera.screen_to_world(at);
            let d = self.text_defaults;
            Element::new(
                self.document.alloc_id(),
                Kind::Text {
                    content: String::new(),
                    font: d.font,
                    size: d.size,
                    align: d.align,
                    bounds: WorldRect::from_corners(
                        p,
                        WorldPt::new(p.x + 1.0, p.y + d.size * 1.25),
                    ),
                },
                self.style,
            )
        });
        let mut hidden = Vec::new();
        if let Some(old) = &before {
            let mut e = old.clone();
            if let Kind::Text { content, .. } = &mut e.kind {
                content.clear();
            }
            hidden.push(e);
        }
        let selection = self.selection.clone();
        self.selection.clear();
        self.text_session = Some(TextSession {
            element,
            before,
            selection,
            hidden,
        });
        Redraw::Tool
    }

    pub(crate) fn update_text(&mut self, value: String, measured: WorldRect) -> Redraw {
        let Some(session) = &mut self.text_session else {
            return Redraw::Nothing;
        };
        if let Kind::Text {
            content, bounds, ..
        } = &mut session.element.kind
        {
            *content = value;
            if measured.min().is_finite() && measured.max().is_finite() {
                *bounds = measured;
            }
        }
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
        self.tool = Tool::Select;
        self.selection = if empty {
            Selection::default()
        } else {
            Selection::only(session.element.id())
        };
        let command = match session.before {
            Some(before) if empty => Command::Remove { id: before.id() },
            Some(before)
                if before.kind == session.element.kind && before.props == session.element.props =>
            {
                return Redraw::Tool;
            }
            Some(before) => Command::Patch {
                id: before.id(),
                before,
                after: session.element,
            },
            None if empty => return Redraw::Tool,
            None => Command::Add {
                element: session.element,
                at: self.document.elements().len(),
            },
        };
        self.execute(command, session.selection);
        Redraw::Scene
    }

    pub(crate) fn set_text_style(&mut self, font: Font, size: f32, align: TextAlign) -> Redraw {
        self.style_text(font, size, align, |content, _, size| {
            let width = content
                .split('\n')
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(1) as f32
                * size
                * 0.6;
            (
                width.max(1.0),
                content.split('\n').count() as f32 * size * 1.25,
            )
        })
    }

    /// Measurement belongs to the app's font engine; the resulting size joins the same command.
    pub fn style_text(
        &mut self,
        font: Font,
        size: f32,
        align: TextAlign,
        mut measure: impl FnMut(&str, Font, f32) -> (f32, f32),
    ) -> Redraw {
        if !size.is_finite() || !(1.0..=1000.0).contains(&size) {
            return Redraw::Nothing;
        }
        self.text_defaults = TextDefaults { font, size, align };
        let change = |element: &mut Element,
                      measure: &mut dyn FnMut(&str, Font, f32) -> (f32, f32)| {
            if let Kind::Text {
                content,
                font: f,
                size: s,
                align: a,
                bounds,
            } = &mut element.kind
            {
                let (w, h) = measure(content, font, size);
                let p = bounds.min();
                *bounds = WorldRect::from_corners(p, WorldPt::new(p.x + w, p.y + h));
                *f = font;
                *s = size;
                *a = align;
            }
        };
        if let Some(session) = &mut self.text_session {
            change(&mut session.element, &mut measure);
            return Redraw::Draft;
        }
        let commands = self
            .selection
            .elements(&self.document)
            .filter(|e| matches!(e.kind, Kind::Text { .. }))
            .filter_map(|before| {
                let mut after = before.clone();
                change(&mut after, &mut measure);
                (after != *before).then(|| Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                })
            })
            .collect::<Vec<_>>();
        if !commands.is_empty() {
            self.execute(Command::Group(commands), self.selection.clone());
        }
        Redraw::Scene
    }

    pub(crate) fn set_roughness(&mut self, roughness: Roughness) -> Redraw {
        self.style.roughness = roughness;
        let commands = self
            .selection
            .elements(&self.document)
            .filter(|e| e.props.roughness != roughness)
            .map(|before| {
                let mut after = before.clone();
                after.props.roughness = roughness;
                Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                }
            })
            .collect::<Vec<_>>();
        if !commands.is_empty() {
            self.execute(Command::Group(commands), self.selection.clone());
        }
        Redraw::Scene
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
