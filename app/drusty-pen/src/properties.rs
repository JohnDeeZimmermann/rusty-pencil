//! Contextual inspector. Each property group owns its controls; edits go through the editor.
mod arrows;
mod colors;
pub(crate) use colors::sync;
mod fill;
mod opacity;
mod settings;
mod stroke;
#[path = "properties/text.rs"]
mod text_controls;

use crate::{State, design as d};
use pen_core::{Property, Value};
use pen_editor::{InputEvent, Tool};
use rune::{El, Tx, col, row, text};

pub(crate) fn panel(s: &State) -> El {
    let editor = &s.editor;
    let dark = editor.settings().dark;
    let selected = !editor.selection().is_empty();
    let mut content = col().gap(d::S3);
    if selected
        || !matches!(editor.tool(), Tool::Select | Tool::Hand)
        || editor.text_session().is_some()
    {
        content = content
            .child(
                text(if selected {
                    "Selection"
                } else {
                    "Tool defaults"
                })
                .font(d::T_LABEL)
                .color(d::themed(d::INK, dark)),
            )
            .child(text_controls::panel(editor))
            .child(colors::group(s, colors::Target::Stroke))
            .child(colors::group(s, colors::Target::Background))
            .child(fill::group(editor))
            .child(stroke::group(editor))
            .child(arrows::group(editor))
            .child(opacity::group(editor));
    }
    content = content.child(settings::group(s));
    d::surface(col().key("shape-properties"), d::R_ISLAND, dark)
        .transition(rune::Transition::none())
        .absolute(d::S4, d::INSPECTOR_TOP)
        .w(d::INSPECTOR_WIDTH)
        .h((editor.viewport().dy - d::PANEL_HEIGHT_MARGIN).max(180.0))
        .padding(d::S3)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
        .child(el_scroll(content))
}

fn group(label: &str, mixed: bool, controls: El, dark: bool) -> El {
    rune_kit::Field::new()
        .label(if mixed {
            format!("{label} · Mixed")
        } else {
            label.into()
        })
        .control(controls.w_full())
        .build(&d::kit(dark))
}

fn choice(key: &str, label: &str, active: bool, property: Property, dark: bool) -> El {
    rune_kit::Button::new(label)
        .on_press_tx(move |s: &mut State, tx| {
            s.input(InputEvent::Property(property), tx);
        })
        .build(&d::kit(dark))
        .key(key)
        .h(d::BUTTON)
        .px(d::S1)
        .min_w(d::BUTTON)
        .radius(d::R_BUTTON)
        .preserve_focus_on_press(true)
        .fill(d::themed(
            if active { d::ACCENT } else { d::ACCENT_SOFT },
            dark,
        ))
        .help(label)
}

fn choices<T: Copy + PartialEq>(
    label: &str,
    key: &str,
    value: Value<T>,
    options: &[(&str, T)],
    property: impl Fn(T) -> Property,
    dark: bool,
) -> El {
    let controls =
        options
            .iter()
            .enumerate()
            .fold(row().w_full().gap(d::S1), |row, (i, (label, v))| {
                row.child(
                    choice(
                        &format!("{key}-{i}"),
                        label,
                        value == Value::Uniform(*v),
                        property(*v),
                        dark,
                    )
                    .w((d::INSPECTOR_WIDTH
                        - 2.0 * d::S3
                        - d::S1 * (options.len() - 1) as f32)
                        / options.len() as f32),
                )
            });
    group(label, value == Value::Mixed, controls, dark)
}

fn el_scroll(content: El) -> El {
    col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .child(content)
}
