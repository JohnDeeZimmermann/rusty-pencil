//! Contextual text controls. Icon buttons preserve the active field's typing focus.
use crate::{State, design as d, property_icons as icons};
use pen_core::{Font, TextAlign};
use pen_editor::{Editor, InputEvent, Tool};
use rune::{El, Shape, Tx, canvas, col, el, row, text};

pub(super) fn panel(editor: &Editor) -> El {
    let has_text = editor.tool() == Tool::Text
        || editor.text_session().is_some()
        || editor.styled_texts().next().is_some();
    if !has_text {
        return el().size(0.0, 0.0);
    }

    let dark = editor.settings().dark;
    let (font, size, align) = editor.text_style();
    let fonts = [
        ("Hand-drawn", Font::HandDrawn),
        ("Normal", Font::Normal),
        ("Code", Font::Code),
    ]
    .into_iter()
    .fold(row().gap(d::S1), |r, (name, value)| {
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
                dark,
            )
            .child(icon(icons::font(value, ink(font == value, dark)))),
        )
    });
    let sizes = [("S", 16.0), ("M", 20.0), ("L", 28.0), ("XL", 36.0)]
        .into_iter()
        .fold(row().gap(d::S1), |r, (name, value)| {
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
                    dark,
                )
                .child(text(name).font(d::T_LABEL).color(d::c(ink(active, dark)))),
            )
        });
    let aligns = [
        ("Left", TextAlign::Left),
        ("Centre", TextAlign::Centre),
        ("Right", TextAlign::Right),
    ]
    .into_iter()
    .fold(row().gap(d::S1), |r, (name, value)| {
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
                dark,
            )
            .child(icon(icons::align(value, ink(align == value, dark)))),
        )
    });
    col()
        .key("properties")
        .transition(rune::Transition::none())
        .gap(d::S3)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
        .child(group("Font family", fonts, dark))
        .child(group("Font size", sizes, dark))
        .child(group("Text align", aligns, dark))
}

fn group(label: &str, controls: El, dark: bool) -> El {
    col()
        .gap(d::S1)
        .child(
            text(label)
                .font(d::T_LABEL)
                .color(d::themed(d::MUTED, dark)),
        )
        .child(controls)
}

fn ink(active: bool, dark: bool) -> pen_core::Srgba {
    let color = if active { d::ON_ACCENT } else { d::INK };
    if dark { color.dark() } else { color }
}

fn icon(shape: Shape) -> El {
    canvas(shape).size(d::ICON, d::ICON)
}

fn choice(key: &str, help: &str, active: bool, event: InputEvent, dark: bool) -> El {
    el().key(key)
        .size(d::BUTTON, d::BUTTON)
        .items_center()
        .justify_center()
        .radius(d::R_BUTTON)
        .fill(d::themed(
            if active { d::ACCENT } else { d::ACCENT_SOFT },
            dark,
        ))
        .hover(if active {
            rune::Color::TRANSPARENT
        } else {
            d::themed(d::BORDER, dark)
        })
        .help(help)
        .preserve_focus_on_press(true)
        .press_scale(d::PRESS)
        .on_press_tx(move |s: &mut State, tx| {
            s.input(event.clone(), tx);
        })
}
