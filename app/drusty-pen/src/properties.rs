//! Contextual inspector. Each property group owns its controls; edits go through the editor.
mod arrows;
mod colors;
pub(crate) use colors::sync;
mod fill;
mod opacity;
pub(crate) mod settings;
mod stroke;
#[path = "properties/text.rs"]
mod text_controls;

use crate::{State, design as d};
use pen_core::{Property, Value};
use pen_editor::{InputEvent, Tool};
use rune::{El, Tx, col, el, row, text};

pub(crate) fn panel(s: &State) -> El {
    // Menus and the inspector share the left column; show one surface at a time.
    if s.editor.modes().view || s.menu.is_some() {
        return el().size(0.0, 0.0);
    }
    let editor = &s.editor;
    let dark = editor.settings().dark;
    let mut content = col().gap(d::S_SECTION);
    if matches!(
        editor.tool(),
        Tool::Frame | Tool::Laser | Tool::Lasso | Tool::Eyedropper
    ) || (editor.selection().len() == 1
        && editor
            .selection()
            .ids()
            .next()
            .and_then(|id| editor.document().get(id))
            .is_some_and(|e| matches!(e.kind, pen_core::Kind::Frame { .. })))
    {
        return el().size(0.0, 0.0);
    }
    if !editor.selection().is_empty()
        || !matches!(editor.tool(), Tool::Select | Tool::Hand)
        || editor.text_session().is_some()
    {
        if editor
            .selection()
            .ids()
            .filter_map(|id| editor.document().get(id))
            .any(|e| matches!(e.kind, pen_core::Kind::Image { .. }))
        {
            content = content.child(crate::images::crop_controls(s));
        }
        let images_only = !editor.selection().is_empty()
            && editor
                .selection()
                .ids()
                .filter_map(|id| editor.document().get(id))
                .all(|e| matches!(e.kind, pen_core::Kind::Image { .. }));
        if !images_only {
            if let Some(text) = text_controls::panel(editor) {
                content = content.child(text);
            }
            if let Some(arrows) = arrows::group(editor) {
                content = content.child(arrows);
            }
            content = content
                .child(colors::group(s, colors::Target::Stroke))
                .child(colors::group(s, colors::Target::Background))
                .child(fill::group(editor))
                .child(stroke::group(editor));
        }
        content = content.child(opacity::group(editor));
    } else {
        return el().size(0.0, 0.0);
    }
    d::surface(col().key("shape-properties"), d::R_ISLAND, dark)
        .transition(rune::Transition::none())
        .absolute(d::S4, d::INSPECTOR_TOP)
        .w(d::INSPECTOR_WIDTH)
        .max_h((editor.viewport().dy - d::PANEL_HEIGHT_MARGIN).max(180.0))
        .padding(d::S3)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
        .child(el_scroll(content))
}

/// Small muted caption above a control. `Mixed` marks fields that differ across the selection.
fn caption(label: &str, mixed: bool, dark: bool) -> El {
    text(if mixed {
        format!("{label} · Mixed")
    } else {
        label.into()
    })
    .font(d::T_CAPTION)
    .color(d::themed(d::MUTED, dark))
}

fn group(label: &str, mixed: bool, controls: El, dark: bool) -> El {
    col()
        .w_full()
        .gap(d::S_LABEL)
        .child(caption(label, mixed, dark))
        .child(controls)
}

/// Width of one segment when `n` share a full-width track.
fn segment_w(n: usize) -> f32 {
    let n = n.max(1) as f32;
    (d::INSPECTOR_INNER - 2.0 * d::SEGMENT_INSET - d::SEGMENT_INSET * (n - 1.0)) / n
}

/// Recessed track holding a segmented control; `rows` are rows of `segment`s.
fn track(rows: El, dark: bool) -> El {
    rows.w_full()
        .gap(d::SEGMENT_INSET)
        .padding(d::SEGMENT_INSET)
        .radius(d::R_BUTTON)
        .fill(d::themed(d::ACCENT_SOFT, dark))
}

/// One option of a segmented control. Active segments use the toolbar's teal state.
fn segment(key: &str, help: &str, active: bool, width: f32, dark: bool) -> El {
    let base = el()
        .key(key)
        .size(width, d::CONTROL - 2.0 * d::SEGMENT_INSET)
        .items_center()
        .justify_center()
        .radius(d::R_CONTROL)
        .help(help)
        .preserve_focus_on_press(true)
        .press_scale(d::PRESS);
    if active {
        base.fill(d::themed(d::ACCENT, dark))
    } else {
        base.hover(d::themed(d::BORDER, dark))
    }
}

/// Ink for content inside a segment.
fn segment_ink(active: bool, dark: bool) -> pen_core::Srgba {
    let color = if active { d::ON_ACCENT } else { d::INK };
    if dark { color.dark() } else { color }
}

/// A text segment that applies `property`. `style` lets glyph labels render larger.
fn choice(
    key: &str,
    label: &str,
    active: bool,
    property: Property,
    (width, style): (f32, rune::tok::TextStyle),
    dark: bool,
) -> El {
    segment(key, label, active, width, dark)
        .child(
            text(label)
                .font(style)
                .color(d::c(segment_ink(active, dark))),
        )
        .on_press_tx(move |s: &mut State, tx| {
            s.input(InputEvent::Property(property), tx);
        })
}

fn choices<T: Copy + PartialEq>(
    label: &str,
    key: &str,
    value: Value<T>,
    options: &[(&str, T)],
    property: impl Fn(T) -> Property,
    dark: bool,
) -> El {
    let width = segment_w(options.len());
    let controls = options
        .iter()
        .enumerate()
        .fold(row(), |row, (i, (label, v))| {
            row.child(choice(
                &format!("{key}-{i}"),
                label,
                value == Value::Uniform(*v),
                property(*v),
                (width, d::T_CONTROL),
                dark,
            ))
        });
    group(label, value == Value::Mixed, track(controls, dark), dark)
}

/// Hugs its content and shrinks to scroll once the panel reaches its maximum height.
fn el_scroll(content: El) -> El {
    rune_kit::popover::fit_scroll(content)
}
