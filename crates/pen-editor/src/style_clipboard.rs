//! Copy/paste styles shares one command, including measured text and container growth.
use crate::{Action, Editor, Redraw, arrange::CopiedStyle, text::place};
use pen_core::{Command, Font, Kind, WorldPt, WorldRect, bound};
use std::collections::BTreeMap;

impl Editor {
    pub(crate) fn copy_style(&mut self) -> Redraw {
        let Some(e) = self.selection.elements(&self.document).next() else {
            return Redraw::Nothing;
        };
        let text = self.styled_texts().next().and_then(|e| match e.kind {
            Kind::Text {
                font, size, align, ..
            } => Some((font, size, align)),
            _ => None,
        });
        self.copied_style = Some(CopiedStyle {
            props: e.props,
            text,
        });
        Redraw::Tool
    }

    /// Text measurements are world-point `(width, height)`, with optional wrap width.
    pub fn paste_style_with(
        &mut self,
        mut measure: impl FnMut(&str, Font, f32, Option<f32>) -> (f32, f32),
    ) -> Redraw {
        if !Action::PasteStyle.enabled(self) {
            return Redraw::Nothing;
        }
        let Some(CopiedStyle { props, text }) = self.copied_style else {
            return Redraw::Nothing;
        };
        let mut changed = BTreeMap::new();
        for e in self.selection.elements(&self.document) {
            let mut after = e.clone();
            after.props = props;
            changed.insert(e.id(), after);
        }
        if let Some((font, size, align)) = text {
            for e in self.styled_texts() {
                let mut after = changed.remove(&e.id()).unwrap_or_else(|| e.clone());
                after.props = props;
                if let Kind::Text {
                    content,
                    font: f,
                    size: s,
                    align: a,
                    ..
                } = &mut after.kind
                {
                    *f = font;
                    *s = size;
                    *a = align;
                    let container = e.kind.container().and_then(|c| self.document.get(c.id()));
                    let wrap = container
                        .and_then(|c| bound::text_box(&c.kind))
                        .map(|b| b.width());
                    let (w, h) = measure(content, font, size, wrap);
                    if let Some(c) = container {
                        if let Some(grown) = place(&mut after, c, (w, h)) {
                            let mut grown = grown;
                            if let Some(styled) = changed.get(&c.id()) {
                                grown.props = styled.props;
                            }
                            changed.insert(c.id(), grown);
                        }
                    } else if let Kind::Text { bounds, .. } = &mut after.kind {
                        let p = bounds.min();
                        *bounds = WorldRect::from_corners(p, WorldPt::new(p.x + w, p.y + h));
                    }
                }
                changed.insert(e.id(), after);
            }
        }
        let commands: Vec<_> = changed
            .into_values()
            .filter_map(|after| {
                let before = self.document.get(after.id())?;
                (before != &after).then(|| Command::Patch {
                    id: after.id(),
                    before: before.clone(),
                    after,
                })
            })
            .collect();
        let redraw = if !commands.is_empty()
            && self.execute(Command::Group(commands), self.selection.clone())
        {
            Redraw::Scene
        } else {
            Redraw::Nothing
        };
        self.refresh_followers();
        redraw
    }
}
