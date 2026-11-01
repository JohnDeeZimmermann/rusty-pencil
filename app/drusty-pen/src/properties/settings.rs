use super::{caption, colors};
use crate::{State, design as d};
use pen_core::Settings;
use pen_editor::InputEvent;
use rune::{El, col, el, row, text};

/// Which boolean setting a switch row toggles.
#[derive(Debug, Clone, Copy)]
enum Toggle {
    ToolLock,
    Grid,
    Dark,
}
impl Toggle {
    fn get(self, s: &Settings) -> bool {
        match self {
            Self::ToolLock => s.tool_lock,
            Self::Grid => s.grid,
            Self::Dark => s.dark,
        }
    }
    fn flip(self, s: &mut Settings) {
        match self {
            Self::ToolLock => s.tool_lock = !s.tool_lock,
            Self::Grid => s.grid = !s.grid,
            Self::Dark => s.dark = !s.dark,
        }
    }
}

pub(crate) fn group(s: &State) -> El {
    let settings = s.editor.settings();
    let dark = settings.dark;
    let mut toggles = col().w_full();
    for (key, label, shortcut, toggle) in [
        ("tool-lock", "Keep tool", "Q", Toggle::ToolLock),
        ("grid-toggle", "Grid & snap", "Ctrl+'", Toggle::Grid),
        ("dark-toggle", "Dark mode", "Alt+Shift+D", Toggle::Dark),
    ] {
        toggles = toggles.child(switch_row(
            key,
            label,
            shortcut,
            toggle.get(&settings),
            dark,
            move |s: &mut State, tx| {
                let mut settings = s.editor.settings();
                toggle.flip(&mut settings);
                s.input(InputEvent::Settings(settings), tx);
            },
        ));
    }
    col()
        .w_full()
        .gap(d::S_SECTION)
        .child(
            col()
                .gap(d::S1)
                .child(caption("Preferences", false, dark))
                .child(toggles),
        )
        .child(colors::group(s, colors::Target::Canvas))
}

/// Full-width row: label and muted shortcut on the left, an on/off switch on the right.
fn switch_row(
    key: &str,
    label: &str,
    shortcut: &str,
    on: bool,
    dark: bool,
    press: impl Fn(&mut State, &mut rune::Tx) + 'static,
) -> El {
    let knob_x = if on {
        d::SWITCH_W - d::SWITCH_H + 2.0
    } else {
        2.0
    };
    let switch = el()
        .size(d::SWITCH_W, d::SWITCH_H)
        .radius(d::SWITCH_H / 2.0)
        .fill(d::themed(if on { d::ON_ACCENT } else { d::BORDER }, dark))
        .inert()
        .child(
            el().absolute(knob_x, 2.0)
                .size(d::SWITCH_H - 4.0, d::SWITCH_H - 4.0)
                .radius((d::SWITCH_H - 4.0) / 2.0)
                .fill(d::themed(d::PAPER, dark)),
        );
    row()
        .key(key)
        .w_full()
        .h(d::CONTROL - d::S1)
        .items_center()
        .justify_between()
        .role("switch")
        .label(label)
        .help(format!("{label} · {shortcut}"))
        .preserve_focus_on_press(true)
        .child(
            row()
                .gap(d::S_LABEL)
                .items_center()
                .inert()
                .child(
                    text(label)
                        .font(d::T_CONTROL)
                        .color(d::themed(d::INK, dark)),
                )
                .child(
                    text(shortcut)
                        .font(d::T_CAPTION)
                        .color(d::themed(d::MUTED, dark)),
                ),
        )
        .child(switch)
        .on_press_tx(press)
}
