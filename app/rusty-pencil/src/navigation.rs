//! Live camera chrome: the return-to-content button stays mounted, hidden and unhittable
//! at zero opacity, so crossing the content boundary never needs a layout or scene rebuild.
use crate::{State, design as d};
use pen_editor::InputEvent;
use rune::{Cursor, El, Live, LiveText, Tx, el, row, text};

pub(crate) fn zoom_pill(label: &LiveText, dark: bool) -> El {
    d::surface(row().key("zoom"), d::R_STATUS, dark)
        .pin(Some(d::S4), None, None, Some(d::S4))
        .padding(d::S1)
        .cursor(Cursor::Default)
        .preserve_focus_on_press(true)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
        .child(zoom_button(
            "−",
            Some(1.0 / 1.2),
            "Zoom out — Ctrl + −",
            dark,
        ))
        .child(
            el().key("zoom-reset")
                .size(64.0, d::BUTTON)
                .items_center()
                .justify_center()
                .radius(d::R_BUTTON)
                .hover(d::themed(d::ACCENT_SOFT, dark))
                .help("Reset zoom — Ctrl + 0")
                .child(
                    text("100%")
                        .live_text(label)
                        .size(64.0, 18.0)
                        .align(rune::Align::Center)
                        .font(d::T_LABEL)
                        .color(d::themed(d::INK, dark)),
                )
                .on_press_tx(|s: &mut State, tx| zoom(s, None, tx)),
        )
        .child(zoom_button("+", Some(1.2), "Zoom in — Ctrl + +", dark))
}

fn zoom_button(label: &str, factor: Option<f32>, help: &str, dark: bool) -> El {
    el().key(format!("zoom-{label}"))
        .size(d::BUTTON, d::BUTTON)
        .items_center()
        .justify_center()
        .radius(d::R_BUTTON)
        .hover(d::themed(d::ACCENT_SOFT, dark))
        .help(help)
        .child(text(label).font(d::T_LABEL).color(d::themed(d::INK, dark)))
        .on_press_tx(move |s: &mut State, tx| zoom(s, factor, tx))
}

fn zoom(s: &mut State, factor: Option<f32>, tx: &mut Tx) {
    // The editor binds zoom commands to the current viewport centre.
    let key = match factor {
        Some(f) if f > 1.0 => '+',
        Some(_) => '-',
        None => '0',
    };
    s.input(
        InputEvent::Key {
            key: pen_editor::Key::Char(key),
            mods: pen_editor::Modifiers {
                ctrl: true,
                ..pen_editor::Modifiers::NONE
            },
        },
        tx,
    );
}

pub(crate) fn content_button(opacity: &Live<f32>, dark: bool) -> El {
    el().key("content-return-position")
        .pin(Some(0.0), None, Some(0.0), Some(d::S4))
        .items_center()
        .justify_center()
        .child(
            d::surface(el().key("content-return"), d::R_BUTTON, dark)
                .live_opacity(opacity)
                .preserve_focus_on_press(true)
                .padding(d::S3)
                .cursor(Cursor::Pointer)
                .hover(d::themed(d::ACCENT_SOFT, dark))
                .press_scale(d::PRESS)
                .child(
                    text("Scroll back to content")
                        .font(d::T_LABEL)
                        .color(d::themed(d::ON_ACCENT, dark)),
                )
                .on_press_tx(|s: &mut State, tx| {
                    s.input(InputEvent::FitContent, tx);
                }),
        )
}
