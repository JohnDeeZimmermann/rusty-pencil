//! Contextual text controls. Icon buttons preserve the active field's typing focus.
use crate::{State, design as d, property_icons as icons};
use pen_core::{Font, TextAlign};
use pen_editor::{Editor, InputEvent, Tool};
use rune::{El, Shape, Tx, canvas, col, el, row, text};

pub(crate) fn panel(editor: &Editor) -> El {
    let has_text = editor.tool() == Tool::Text
        || editor.text_session().is_some()
        || editor.styled_texts().next().is_some();
    if !has_text {
        return el().size(0.0, 0.0);
    }

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
            )
            .child(icon(icons::font(value, ink(font == value)))),
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
                )
                .child(text(name).font(d::T_LABEL).color(d::c(ink(active)))),
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
            )
            .child(icon(icons::align(value, ink(align == value)))),
        )
    });
    d::raised(col().key("properties"), d::R_ISLAND)
        .transition(rune::Transition::none())
        .absolute(d::S4, d::INSPECTOR_TOP)
        .w(d::INSPECTOR_WIDTH)
        .padding(d::S3)
        .gap(d::S3)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
        .child(group("Font family", fonts))
        .child(group("Font size", sizes))
        .child(group("Text align", aligns))
}

fn group(label: &str, controls: El) -> El {
    col()
        .gap(d::S1)
        .child(text(label).font(d::T_LABEL).color(d::c(d::MUTED)))
        .child(controls)
}

fn ink(active: bool) -> pen_core::Srgba {
    if active { d::ON_ACCENT } else { d::INK }
}

fn icon(shape: Shape) -> El {
    canvas(shape).size(d::ICON, d::ICON)
}

fn choice(key: &str, help: &str, active: bool, event: InputEvent) -> El {
    el().key(key)
        .size(d::BUTTON, d::BUTTON)
        .items_center()
        .justify_center()
        .radius(d::R_BUTTON)
        .fill(d::c(if active { d::ACCENT } else { d::ACCENT_SOFT }))
        .hover(if active {
            rune::Color::TRANSPARENT
        } else {
            d::c(d::BORDER)
        })
        .help(help)
        .preserve_focus_on_press(true)
        .press_scale(d::PRESS)
        .on_press_tx(move |s: &mut State, tx| {
            s.input(event.clone(), tx);
        })
}
