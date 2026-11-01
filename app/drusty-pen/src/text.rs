//! The font engine measures world geometry; Rune's field owns caret, selection and IME.
use crate::{State, design as d, layer, paint as shapes};
use pen_core::{Font, Kind, TextAlign, WorldPt, WorldRect};
use pen_editor::InputEvent;
use pen_render::Prim;
use rune::text::TextSystem;
use rune::{Align, El, Family, Key, TextEdit, Tx, el, text, text_input, tok::TextStyle};

pub(crate) static FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/Inter.ttf"),
    include_bytes!("../../../assets/fonts/Caveat.ttf"),
    include_bytes!("../../../assets/fonts/RobotoMono.ttf"),
];
// Rune bounds inherited glyph raster scale to 1/16..=4. Give document text an
// equivalent 8x layout with a compensating transform, so 3000% reaches only 3.75x.
// This preserves live-only camera updates and rasterizes at the actual screen size.
const TEXT_LAYOUT_SCALE: f32 = 8.0;

fn display_style(font: Font, size: f32) -> TextStyle {
    style(font, size * TEXT_LAYOUT_SCALE).optical_size(size)
}

pub(crate) const FAMILIES: (&str, &str) = ("Inter", "Roboto Mono");

pub(crate) fn system() -> TextSystem {
    TextSystem::new(FONTS, FAMILIES)
}

pub(crate) fn style(font: Font, size: f32) -> TextStyle {
    TextStyle::new(size, 400.0)
        .lh(size * 1.25)
        .family(Family::Named(match font {
            Font::HandDrawn => "Caveat",
            Font::Normal => "Inter",
            Font::Code => "Roboto Mono",
        }))
}
fn alignment(align: TextAlign) -> Align {
    match align {
        TextAlign::Left => Align::Start,
        TextAlign::Centre => Align::Center,
        TextAlign::Right => Align::End,
    }
}

pub(crate) fn measure(system: &mut TextSystem, content: &str, font: Font, size: f32) -> (f32, f32) {
    let width = content
        .split('\n')
        .map(|line| system.measure(line, style(font, size), 1.0).0)
        .fold(1.0_f32, f32::max);
    system.end_commit();
    (
        width + 2.0,
        content.split('\n').count() as f32 * size * 1.25,
    )
}

/// Partition around text so filled shapes still occlude text below them in document order.
pub(crate) fn paint(prims: &[Prim]) -> El {
    let mut root = el()
        .absolute(0.0, 0.0)
        .size(0.0, 0.0)
        .instant_changes()
        .inert();
    let mut start = 0;
    for (i, prim) in prims.iter().enumerate() {
        if let Prim::Text {
            content,
            font,
            size,
            align,
            bounds,
            color,
        } = prim
        {
            if start < i {
                let (shape, bounds) = shapes::bounded(&prims[start..i]);
                root = root.child(layer(shape, bounds, &format!("paint-{start}")));
            }
            root = root.child(
                text(content.clone())
                    .key(format!("text-{i}"))
                    .absolute(bounds.min().x, bounds.min().y)
                    .size(
                        bounds.width() * TEXT_LAYOUT_SCALE,
                        bounds.height() * TEXT_LAYOUT_SCALE,
                    )
                    .scale(1.0 / TEXT_LAYOUT_SCALE)
                    .origin(0.0, 0.0)
                    .font(display_style(*font, *size))
                    .align(alignment(*align))
                    .color(d::c(*color)),
            );
            start = i + 1;
        }
    }
    if start < prims.len() {
        let (shape, bounds) = shapes::bounded(&prims[start..]);
        root = root.child(layer(shape, bounds, &format!("paint-{start}")));
    }
    root
}

pub(crate) fn editor(s: &State) -> El {
    let Some(session) = s.editor.text_session() else {
        return el().size(0.0, 0.0);
    };
    let Kind::Text {
        font,
        size,
        align,
        bounds,
        ..
    } = &session.element.kind
    else {
        return el();
    };
    let field = text_input(&s.text_edit, display_style(*font, *size))
        .key("canvas-text-editor")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .instant_changes()
        .color(d::c(session.element.props.stroke))
        .caret(d::c(d::ON_ACCENT), d::c(d::SELECTION_WASH))
        .align(alignment(*align))
        .on_edit(|s: &mut State| &mut s.text_edit)
        .on_input_change(|s: &mut State, tx| s.sync_text(tx))
        .on_submit(|s: &mut State, tx| {
            s.text_edit.insert("\n");
            s.sync_text(tx);
        })
        .on_input_key(|s: &mut State, event, tx| {
            if event.key == Key::Escape {
                s.sync_text(tx);
                s.input(InputEvent::FinishText, tx);
                tx.blur();
                return true;
            }
            if crate::files::key(s, event, tx) {
                return true;
            }
            if event.mods.cmd
                && event.mods.shift
                && matches!(
                    event.key,
                    Key::Command('<' | '>' | ',' | '.') | Key::Text(_)
                )
            {
                return crate::on_key(s, event, tx);
            }
            false
        });
    // The invisible label lets Rune lay out the full IME preedit in this frame, before it
    // becomes committed content. The overlay field shares its intrinsic size and position.
    el().key("text-edit-layout")
        .absolute(bounds.min().x, bounds.min().y)
        .min_w(bounds.width().max(3.0) * TEXT_LAYOUT_SCALE)
        .min_h(bounds.height().max(*size * 1.25) * TEXT_LAYOUT_SCALE)
        .scale(1.0 / TEXT_LAYOUT_SCALE)
        .origin(0.0, 0.0)
        .instant_changes()
        .child(
            text(s.text_edit.display().0)
                .font(display_style(*font, *size))
                .color(rune::Color::TRANSPARENT)
                .inert(),
        )
        .child(field)
}

impl State {
    pub(crate) fn sync_text(&mut self, tx: &mut Tx) {
        let Some(session) = self.editor.text_session() else {
            return;
        };
        let Kind::Text {
            font, size, bounds, ..
        } = session.element.kind
        else {
            return;
        };
        // Composition remains in TextEdit until committed by the platform IME.
        let (w, h) = measure(&mut self.fonts, &self.text_edit.text, font, size);
        let p = bounds.min();
        self.input(
            InputEvent::TextChanged {
                content: self.text_edit.text.clone(),
                bounds: WorldRect::from_corners(p, WorldPt::new(p.x + w, p.y + h)),
            },
            tx,
        );
    }

    pub(crate) fn text_session_changed(&mut self, old: Option<pen_core::ElementId>) {
        let session = self.editor.text_session();
        if session.map(|s| s.element.id()) == old {
            return;
        }
        self.text_edit = session.map_or_else(TextEdit::default, |session| {
            let Kind::Text { content, .. } = &session.element.kind else {
                return TextEdit::default();
            };
            let mut edit = TextEdit::new(content.clone());
            edit.focused = true;
            edit
        });
    }
}
