use super::*;
use crate::{
    State,
    tests::{FRAME, app, drag},
};
use pen_editor::ScreenPt;
use rune::{Runtime, Tx};

fn scene(rt: &Runtime<State>) -> Vec<u8> {
    pen_core::file::save(&rt.state.snapshot()).unwrap()
}

fn image() -> Vec<u8> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/pen-core/tests/fixtures/landscape.png");
    std::fs::read(fixture).unwrap()
}

#[test]
fn app_data_round_trips_and_removing_missing_data_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/preferences.json");
    assert!(!exists(&path));
    write(&path, b"{\"dark\":true}").unwrap();
    assert!(exists(&path));
    assert_eq!(read(&path).unwrap(), b"{\"dark\":true}");
    remove(&path).unwrap();
    assert!(!exists(&path));
    remove(&path).unwrap();
}

#[test]
fn dropped_bytes_open_scenes_in_tabs_and_insert_images_where_dropped() {
    let mut rt = app();
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    let bytes = scene(&rt);
    let files = vec![
        Picked {
            path: "board.rusty-pencil".into(),
            bytes,
        },
        Picked {
            path: "photo.png".into(),
            bytes: image(),
        },
    ];
    crate::images::drop_picked(
        &mut rt.state,
        files,
        ScreenPt::new(500.0, 400.0),
        &mut Tx::default(),
    );
    rt.frame(FRAME);
    assert_eq!(
        rt.state.tabs.entries.len(),
        2,
        "the scene opened in a new tab"
    );
    let elements = rt.state.editor.document().elements();
    assert_eq!(elements.len(), 2, "the image joined the opened scene");
    assert!(matches!(elements[1].kind, pen_core::Kind::Image { .. }));
    assert!(rt.state.files.title().ends_with("board.rusty-pencil"));
}

#[test]
fn a_picked_file_that_is_not_a_scene_reports_and_opens_nothing() {
    let mut rt = app();
    let picked = Picked {
        path: "notes.json".into(),
        bytes: b"garbage".to_vec(),
    };
    rt.state.open_picked(&picked, &mut Tx::default());
    rt.frame(FRAME);
    assert_eq!(rt.state.tabs.entries.len(), 1);
    assert!(rt.state.editor.document().elements().is_empty());
}

#[test]
fn desktop_tab_shortcuts_keep_the_command_key() {
    let mut rt = app();
    let press = |rt: &mut Runtime<State>, mods: rune::Mods| {
        rt.key(rune::KeyEvent {
            key: rune::Key::Command('t'),
            mods,
        });
        rt.frame(FRAME);
    };
    press(
        &mut rt,
        rune::Mods {
            alt: true,
            ..rune::Mods::default()
        },
    );
    assert_eq!(
        rt.state.tabs.entries.len(),
        1,
        "Alt+T is the browser binding"
    );
    press(
        &mut rt,
        rune::Mods {
            cmd: true,
            ..rune::Mods::default()
        },
    );
    assert_eq!(rt.state.tabs.entries.len(), 2);
    assert_eq!(
        (TAB_KEYS.new, TAB_KEYS.close, TAB_KEYS.next),
        ("Ctrl+T", "Ctrl+W", "Ctrl+Tab")
    );
}
