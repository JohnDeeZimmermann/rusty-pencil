//! Application menus reuse the inspector and toolbar's surfaces and controls.
use crate::{State, design as d, files, properties};
use rune::{Cursor, El, Tx, el, row, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    File,
    Preferences,
}

pub(crate) fn view(s: &State) -> El {
    let dark = s.editor.settings().dark;
    let mut layer = el()
        .key("app-menu-layer")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full();
    if let Some(section) = s.menu {
        // Consume the entire dismissal gesture so it cannot draw on the canvas.
        layer = layer.on_press(|s: &mut State| s.menu = None).child(
            d::surface(el().key("app-menu-popup"), d::R_ISLAND, dark)
                .instant_changes()
                .absolute(d::S4, d::APP_MENU_TOP)
                .w(d::INSPECTOR_WIDTH)
                .padding(d::S3)
                .cursor(Cursor::Default)
                .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
                .child(match section {
                    Section::File => files::menu(s),
                    Section::Preferences => properties::settings::group(s),
                }),
        );
    }
    layer.child(
        d::surface(row().key("app-menu"), d::R_ISLAND, dark)
            .absolute(d::S4, d::S4)
            .padding(d::PADDING)
            .gap(d::S1)
            .cursor(Cursor::Default)
            .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
            .child(tab(
                s,
                Section::File,
                "File",
                "menu-file",
                d::FILE_TAB_WIDTH,
            ))
            .child(tab(
                s,
                Section::Preferences,
                "Preferences",
                "menu-preferences",
                d::PREFERENCES_TAB_WIDTH,
            )),
    )
}

fn tab(s: &State, section: Section, label: &str, key: &str, width: f32) -> El {
    let dark = s.editor.settings().dark;
    let active = s.menu == Some(section);
    el().key(key)
        .size(width, d::CONTROL)
        .radius(d::R_BUTTON)
        .role("button")
        .label(label)
        .help(format!("{label} menu"))
        .items_center()
        .justify_center()
        .fill(d::themed(if active { d::ACCENT } else { d::PAPER }, dark))
        .hover(d::themed(d::ACCENT_SOFT, dark))
        .child(
            text(label)
                .font(d::T_LABEL)
                .color(d::themed(if active { d::ON_ACCENT } else { d::INK }, dark)),
        )
        .on_press_tx(move |s: &mut State, tx| {
            if s.editor.text_session().is_some() {
                s.sync_text(tx);
                s.input(pen_editor::InputEvent::FinishText, tx);
            }
            s.context = None;
            s.menu = if s.menu == Some(section) {
                None
            } else {
                Some(section)
            };
            tx.blur();
        })
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
