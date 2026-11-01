use crate::{
    State,
    tests::{FRAME, app, drag},
};
use pen_editor::Camera;
use rune::{Key, KeyEvent, Mods, Runtime, Tx};

fn chord(rt: &mut Runtime<State>, c: char) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}
fn toggle(rt: &mut Runtime<State>) {
    rt.key(KeyEvent {
        key: Key::Text("d".into()),
        mods: Mods {
            alt: true,
            shift: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}
fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).expect(key);
    drag(rt, (x, y), (x, y));
}

#[test]
fn theme_is_global_across_existing_and_new_tabs_and_does_not_dirty_canvases() {
    let mut rt = app();
    chord(&mut rt, 't');
    toggle(&mut rt);
    assert!(rt.state.preferences.dark && rt.state.editor.settings().dark);
    assert!(!rt.state.files.dirty);
    chord(&mut rt, '1');
    assert!(rt.state.editor.settings().dark);
    assert!(!rt.state.files.dirty);
    chord(&mut rt, 't');
    assert!(rt.state.editor.settings().dark);
    click(&mut rt, "menu-preferences");
    click(&mut rt, "dark-toggle");
    assert!(!rt.state.preferences.dark);
    assert!(!rt.state.files.dirty);
    chord(&mut rt, '1');
    assert!(!rt.state.editor.settings().dark);
    chord(&mut rt, '2');
    assert!(!rt.state.editor.settings().dark);
}

#[test]
fn opening_and_resetting_scenes_preserve_the_global_theme() {
    let mut rt = app();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.drusty-pen");
    let mut value = serde_json::to_value(rt.state.snapshot()).unwrap();
    value["app_state"]["settings"]["dark"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    rt.drop_files(vec![path.clone()], None);
    rt.frame(FRAME);
    assert!(
        !rt.state.editor.settings().dark,
        "legacy file appearance cannot override the editor"
    );
    toggle(&mut rt);
    let light = dir.path().join("light.drusty-pen");
    crate::files::write(&light, &rt.state.snapshot()).unwrap();
    rt.drop_files(vec![light], None);
    rt.frame(FRAME);
    assert!(rt.state.editor.settings().dark);
    click(&mut rt, "menu-file");
    click(&mut rt, "reset-canvas");
    click(&mut rt, "confirm-reset");
    assert!(rt.state.editor.settings().dark);
    let bytes = pen_core::file::save(&rt.state.snapshot()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value["app_state"]["settings"].get("dark").is_none());
}

#[test]
fn global_theme_survives_restart_independently_of_workspace_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let mut rt = app();
    rt.state.tabs.workspace = Some(dir.path().join("workspace.json"));
    rt.state.files.recovery = Some(dir.path().join("autosave.drusty-pen"));
    toggle(&mut rt);
    assert!(dir.path().join("preferences.json").exists());
    chord(&mut rt, 't');
    crate::files::close(&mut rt.state, &mut Tx::default());
    let mut restored = State::new(Camera::default());
    restored.tabs.workspace = rt.state.tabs.workspace.clone();
    restored.restore_preferences();
    assert!(restored.restore_workspace());
    assert!(restored.preferences.dark && restored.editor.settings().dark);
    restored.switch_tab(crate::tabs::TabId(1), &mut Tx::default());
    assert!(restored.editor.settings().dark);
    std::fs::write(dir.path().join("workspace.json"), b"broken").unwrap();
    let mut independent = State::new(Camera::default());
    independent.tabs.workspace = rt.state.tabs.workspace.clone();
    independent.restore_preferences();
    assert!(!independent.restore_workspace());
    assert!(independent.editor.settings().dark);
}
