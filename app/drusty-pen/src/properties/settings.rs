use super::{colors, group as field};
use crate::{State, design as d};
use pen_editor::InputEvent;
use rune::{El, col};
pub(super) fn group(s: &State) -> El {
    let settings = s.editor.settings();
    let mut controls = col().gap(d::S1);
    for (key, label, on) in [
        ("tool-lock", "Keep tool · Q", settings.tool_lock),
        ("grid-toggle", "Grid & snap · Ctrl+'", settings.grid),
        ("dark-toggle", "Dark mode · Alt+Shift+D", settings.dark),
    ] {
        controls = controls.child(
            rune_kit::Button::new(label)
                .on_press_tx(move |s: &mut State, tx| {
                    let mut settings = s.editor.settings();
                    match key {
                        "tool-lock" => settings.tool_lock = !settings.tool_lock,
                        "grid-toggle" => settings.grid = !settings.grid,
                        _ => settings.dark = !settings.dark,
                    }
                    s.input(InputEvent::Settings(settings), tx);
                })
                .build(&d::kit(settings.dark))
                .key(key)
                .h(d::BUTTON)
                .w_full()
                .preserve_focus_on_press(true)
                .fill(d::themed(
                    if on { d::ACCENT } else { d::ACCENT_SOFT },
                    settings.dark,
                ))
                .help(label),
        );
    }
    col()
        .gap(d::S3)
        .child(field("Canvas settings", false, controls, settings.dark))
        .child(colors::group(s, colors::Target::Canvas))
}
