//! Text and sloppiness styling. Bound text is styled through its selected container.
use crate::text::{TextDefaults, place};
use crate::{Editor, Redraw, Selection};
use pen_core::{Command, Document, Element, ElementId, Font, Kind, Roughness, TextAlign};
use pen_core::{WorldPt, WorldRect, bound};

/// Texts the inspector styles: selected free text and the text of selected containers.
/// Looked up when the selection or document changes, never per frame.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextFocus {
    key: Option<(u64, Selection)>,
    ids: Vec<ElementId>,
}

impl TextFocus {
    pub(crate) fn update(&mut self, doc: &Document, selection: &Selection) {
        if self
            .key
            .as_ref()
            .is_some_and(|(revision, s)| *revision == doc.revision() && s == selection)
        {
            return;
        }
        self.ids.clear();
        if !selection.is_empty() {
            self.ids.extend(
                doc.elements()
                    .iter()
                    .filter(|e| match e.kind {
                        Kind::Text {
                            container: None, ..
                        } => selection.contains(e.id()),
                        Kind::Text {
                            container: Some(c), ..
                        } => selection.contains(c.id()),
                        _ => false,
                    })
                    .map(Element::id),
            );
        }
        self.key = Some((doc.revision(), selection.clone()));
    }
}

impl Editor {
    #[must_use]
    pub fn text_defaults(&self) -> (Font, f32, TextAlign) {
        let d = self.text_defaults;
        (d.font, d.size, d.align)
    }

    /// Restores validated defaults at a scene-file boundary without editing existing text.
    pub fn restore_text_defaults(&mut self, font: Font, size: f32, align: TextAlign) {
        self.text_defaults = TextDefaults { font, size, align };
    }

    /// Committed texts the text controls act on, in document order.
    pub fn styled_texts(&self) -> impl Iterator<Item = &Element> + '_ {
        self.text_focus
            .ids
            .iter()
            .filter_map(|id| self.document.get(*id))
    }

    #[must_use]
    pub fn text_style(&self) -> (Font, f32, TextAlign) {
        let element = self
            .text_session
            .as_ref()
            .map(|s| &s.element)
            .or_else(|| self.styled_texts().next());
        if let Some(Element {
            kind: Kind::Text {
                font, size, align, ..
            },
            ..
        }) = element
        {
            (*font, *size, *align)
        } else {
            self.text_defaults()
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

    /// A monospace-ish estimate for tests and headless use; the app measures real glyphs.
    pub(crate) fn set_text_style(&mut self, font: Font, size: f32, align: TextAlign) -> Redraw {
        self.style_text(font, size, align, |content, _, size, wrap| {
            let longest = content
                .split('\n')
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(1) as f32
                * size
                * 0.6;
            let lines: f32 = content
                .split('\n')
                .map(|s| match wrap {
                    Some(w) if w > 0.0 => {
                        (s.chars().count() as f32 * size * 0.6 / w).ceil().max(1.0)
                    }
                    _ => 1.0,
                })
                .sum();
            (
                wrap.map_or(longest, |w| longest.min(w)).max(1.0),
                lines * size * 1.25,
            )
        })
    }

    /// Measurement belongs to the app's font engine: `measure(content, font, size, wrap)`
    /// returns the laid-out `(width, height)` in world points, wrapping at `wrap` when given.
    /// Resized text and any container grown to fit join the same command.
    pub fn style_text(
        &mut self,
        font: Font,
        size: f32,
        align: TextAlign,
        mut measure: impl FnMut(&str, Font, f32, Option<f32>) -> (f32, f32),
    ) -> Redraw {
        if self.selection.elements(&self.document).any(|e| e.locked) {
            return Redraw::Nothing;
        }
        if !size.is_finite() || !(1.0..=1000.0).contains(&size) {
            return Redraw::Nothing;
        }
        self.text_defaults = TextDefaults { font, size, align };
        // Restyles `text` and returns the measured size it now needs.
        let mut restyle = |text: &mut Element, container: Option<&Element>| -> (f32, f32) {
            let wrap = container
                .and_then(|c| bound::text_box(&c.kind))
                .map(|r| r.width());
            let Kind::Text {
                content,
                font: f,
                size: s,
                align: a,
                ..
            } = &mut text.kind
            else {
                return (0.0, 0.0);
            };
            *f = font;
            *s = size;
            *a = align;
            measure(content, font, size, wrap)
        };
        if let Some(session) = &mut self.text_session {
            let container = session.container().cloned();
            let (w, h) = restyle(&mut session.element, container.as_ref());
            let p = session.element.kind.bounds().min();
            session.resize(WorldRect::from_corners(p, WorldPt::new(p.x + w, p.y + h)));
            self.refresh_followers();
            return Redraw::Draft;
        }
        let mut commands = Vec::new();
        for before in self.styled_texts() {
            let container = before
                .kind
                .container()
                .and_then(|c| self.document.get(c.id()));
            let mut after = before.clone();
            let (w, h) = restyle(&mut after, container);
            let grown = match container {
                Some(c) => place(&mut after, c, (w, h)),
                None => {
                    if let Kind::Text { bounds, .. } = &mut after.kind {
                        let p = bounds.min();
                        *bounds = WorldRect::from_corners(p, WorldPt::new(p.x + w, p.y + h));
                    }
                    None
                }
            };
            if after != *before {
                commands.push(Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                });
            }
            if let (Some(before), Some(after)) = (container, grown) {
                commands.push(Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                });
            }
        }
        if !commands.is_empty() {
            self.execute(Command::Group(commands), self.selection.clone());
        }
        self.refresh_followers();
        Redraw::Scene
    }

    pub(crate) fn set_roughness(&mut self, roughness: Roughness) -> Redraw {
        if self.selection.elements(&self.document).any(|e| e.locked) {
            return Redraw::Nothing;
        }
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

impl Editor {
    /// Actual bound text alignment, including mixed selections and an active text edit.
    #[must_use]
    pub fn vertical_align(&self) -> pen_core::Value<pen_core::VerticalAlign> {
        if let Some(session) = &self.text_session {
            return pen_core::Value::Uniform(session.element.props.vertical_align);
        }
        let mut texts = self.styled_texts();
        let Some(first) = texts.next() else {
            return self.property(|p| p.vertical_align);
        };
        let align = first.props.vertical_align;
        if texts.all(|e| e.props.vertical_align == align) {
            pen_core::Value::Uniform(align)
        } else {
            pen_core::Value::Mixed
        }
    }
}
