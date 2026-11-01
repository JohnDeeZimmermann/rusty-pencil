//! Contextual text controls. Icon buttons preserve the active field's typing focus.
use super::{group, segment, segment_ink as ink, segment_w, track};
use crate::{State, design as d, property_icons as icons};
use pen_core::{Font, TextAlign};
use pen_editor::{Editor, InputEvent, Tool};
use rune::{El, Shape, Tx, canvas, col, row, text};

pub(super) fn panel(editor: &Editor) -> Option<El> {
    let has_text = editor.tool() == Tool::Text
        || editor.text_session().is_some()
        || editor.styled_texts().next().is_some();
    if !has_text {
        return None;
    }

    let dark = editor.settings().dark;
    let (font, size, align) = editor.text_style();
    let fonts = [
        ("Hand-drawn", Font::HandDrawn),
        ("Normal", Font::Normal),
        ("Code", Font::Code),
    ]
    .into_iter()
    .fold(row(), |r, (name, value)| {
        r.child(
            choice(
                &format!("font-{value:?}"),
                name,
                font == value,
                InputEvent::TextStyle {
                    font: value,
                    size,
                    align,
                },
                segment_w(3),
                dark,
            )
            .child(icon(icons::font(value, ink(font == value, dark)))),
        )
    });
    let sizes = [("S", 16.0), ("M", 20.0), ("L", 28.0), ("XL", 36.0)]
        .into_iter()
        .fold(row(), |r, (name, value)| {
            let active = (size - value).abs() < 0.1;
            r.child(
                choice(
                    &format!("size-{name}"),
                    &format!("Font size {name} · {value:.0} pt"),
                    active,
                    InputEvent::TextStyle {
                        font,
                        size: value,
                        align,
                    },
                    segment_w(4),
                    dark,
                )
                .child(text(name).font(d::T_CONTROL).color(d::c(ink(active, dark)))),
            )
        });
    let aligns = [
        ("Left", TextAlign::Left),
        ("Centre", TextAlign::Centre),
        ("Right", TextAlign::Right),
    ]
    .into_iter()
    .fold(row(), |r, (name, value)| {
        r.child(
            choice(
                &format!("align-{name}"),
                &format!("Align {name}"),
                align == value,
                InputEvent::TextStyle {
                    font,
                    size,
                    align: value,
                },
                segment_w(3),
                dark,
            )
            .child(icon(icons::align(value, ink(align == value, dark)))),
        )
    });
    let valign = editor.vertical_align();
    let vertical = [
        ("Top", pen_core::VerticalAlign::Top),
        ("Middle", pen_core::VerticalAlign::Middle),
        ("Bottom", pen_core::VerticalAlign::Bottom),
    ]
    .into_iter()
    .fold(row(), |r, (name, value)| {
        r.child(
            choice(
                &format!("vertical-{name}"),
                name,
                valign == pen_core::Value::Uniform(value),
                InputEvent::Property(pen_core::Property::VerticalAlign(value)),
                segment_w(3),
                dark,
            )
            .child(
                text(name)
                    .font(d::T_CONTROL)
                    .color(d::c(ink(valign == pen_core::Value::Uniform(value), dark))),
            ),
        )
    });
    Some(
        col()
            .key("properties")
            .transition(rune::Transition::none())
            .gap(d::S_SECTION)
            .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
            .child(group("Font family", false, track(fonts, dark), dark))
            .child(group("Font size", false, track(sizes, dark), dark))
            .child(group("Text align", false, track(aligns, dark), dark))
            .child(group(
                "Vertical align",
                valign == pen_core::Value::Mixed,
                track(vertical, dark),
                dark,
            )),
    )
}

fn icon(shape: Shape) -> El {
    canvas(shape).size(d::ICON, d::ICON).inert()
}

fn choice(key: &str, help: &str, active: bool, event: InputEvent, width: f32, dark: bool) -> El {
    segment(key, help, active, width, dark).on_press_tx(move |s: &mut State, tx| {
        s.input(event.clone(), tx);
    })
}
