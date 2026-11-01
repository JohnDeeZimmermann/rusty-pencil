use crate::{
    State,
    menu::Section,
    tests::{FRAME, app, drag, key},
};
use rune::{FileDialogKind, Key, KeyEvent, Mods, Runtime};

fn click(rt: &mut Runtime<State>, name: &str) {
    let (x, y) = rt.find(name).expect(name);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(FRAME);
}

#[test]
fn menus_switch_toggle_and_dismiss_without_drawing() {
    let mut rt = app();
    assert!(rt.find("open-file").is_none());
    assert!(rt.find("grid-toggle").is_none());
    click(&mut rt, "menu-file");
    assert!(rt.find("open-file").is_some());
    click(&mut rt, "menu-preferences");
    assert!(rt.find("open-file").is_none());
    assert!(rt.find("Canvas-hex").is_some());
    click(&mut rt, "tool-lock");
    assert!(rt.state.editor.settings().tool_lock);
    click(&mut rt, "menu-preferences");
    assert!(rt.state.menu.is_none());
    click(&mut rt, "menu-file");
    drag(&mut rt, (700.0, 300.0), (800.0, 400.0));
    assert!(rt.state.menu.is_none());
    assert!(rt.state.editor.document().elements().is_empty());
    click(&mut rt, "menu-preferences");
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert!(rt.find("app-menu-popup").is_none());
    key(&mut rt, "r");
    drag(&mut rt, (700.0, 300.0), (800.0, 400.0));
    assert_eq!(rt.state.editor.document().elements().len(), 1);
}

#[test]
fn file_actions_close_menu_and_open_existing_dialogs() {
    let mut rt = app();
    click(&mut rt, "menu-file");
    click(&mut rt, "open-file");
    assert!(rt.state.menu.is_none());
    assert!(rt.find("app-menu-popup").is_none());
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Open);
    rt.resolve_file_dialog(None).unwrap();
    rt.frame(FRAME);
    click(&mut rt, "menu-file");
    click(&mut rt, "save-file");
    assert!(rt.state.menu.is_none());
    assert!(rt.find("app-menu-popup").is_none());
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Save);
    rt.resolve_file_dialog(None).unwrap();
    rt.frame(FRAME);
    click(&mut rt, "menu-file");
    assert_eq!(rt.state.menu, Some(Section::File));
    click(&mut rt, "reset-canvas");
    assert!(rt.state.menu.is_none());
    assert!(rt.find("confirm-reset").is_some());
}

#[test]
fn opening_a_menu_finishes_text_and_escape_closes_a_focused_colour_field() {
    let mut rt = app();
    key(&mut rt, "t");
    rt.pointer_down(400.0, 300.0);
    rt.pointer_up(400.0, 300.0);
    rt.frame(FRAME);
    key(&mut rt, "Menu test");
    click(&mut rt, "menu-preferences");
    assert!(rt.state.editor.text_session().is_none());
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    assert!(rt.find("Canvas-hex").is_some());
    click(&mut rt, "Canvas-hex");
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert!(rt.state.menu.is_none());
    assert!(rt.find("app-menu-popup").is_none());
}
