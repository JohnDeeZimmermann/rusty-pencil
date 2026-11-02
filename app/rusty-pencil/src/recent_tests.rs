use super::*;
use crate::tests::{FRAME, app, drag, key as type_key};
use pen_editor::Camera;
use rune::{KeyEvent, Mods, Runtime};

fn click(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).expect(key);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(FRAME);
}

fn key(rt: &mut Runtime<State>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}

fn scene(path: &Path) {
    let s = State::new(Camera::default());
    files::write(path, &s.snapshot()).unwrap();
}

#[test]
fn successful_opens_persist_in_most_recent_order_without_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("First.rusty-pencil");
    let b = dir.path().join("Second.rusty-pencil");
    scene(&a);
    scene(&b);
    let mut s = State::new(Camera::default());
    s.tabs.workspace = Some(dir.path().join("workspace.json"));
    s.open_path(&a, &mut Tx::default());
    s.open_path(&b, &mut Tx::default());
    s.open_path(&dir.path().join("./First.rusty-pencil"), &mut Tx::default());
    assert_eq!(s.recent.paths, [a.clone(), b.clone()]);
    let mut restored = State::new(Camera::default());
    restored.tabs.workspace = s.tabs.workspace;
    restored.restore_recent();
    assert_eq!(restored.recent.paths, [a, b]);
}

#[test]
fn recent_history_is_bounded_and_ignores_failed_opens() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = State::new(Camera::default());
    for i in 0..LIMIT + 3 {
        s.remember_recent(&dir.path().join(format!("{i}.rusty-pencil")), &s.snapshot());
    }
    let expected = s.recent.paths.clone();
    s.open_path(&dir.path().join("missing.rusty-pencil"), &mut Tx::default());
    let corrupt = dir.path().join("corrupt.rusty-pencil");
    std::fs::write(&corrupt, b"not a scene").unwrap();
    s.open_path(&corrupt, &mut Tx::default());
    assert_eq!(s.recent.paths.len(), LIMIT);
    assert_eq!(s.recent.paths, expected);
    assert!(s.recent.paths[0].ends_with("12.rusty-pencil"));
}

#[test]
fn saving_through_the_file_dialog_adds_a_recent_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Saved idea.rusty-pencil");
    let mut rt = app();
    rt.key(KeyEvent {
        key: Key::Command('s'),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert_eq!(rt.state.recent.paths, [path]);
}

#[test]
fn menu_reopens_a_closed_file_without_replacing_unsaved_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("An idea.rusty-pencil");
    scene(&path);
    let mut rt = app();
    rt.state.open_path(&path, &mut Tx::default());
    rt.state
        .request_close_tab(rt.state.tabs.active, &mut Tx::default());
    type_key(&mut rt, "r");
    drag(&mut rt, (500.0, 300.0), (600.0, 400.0));
    let unsaved = rt.state.tabs.active;
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    click(&mut rt, "recent-file-0");
    assert!(rt.state.menu.is_none());
    assert_eq!(rt.state.files.path.as_deref(), Some(path.as_path()));
    rt.state.switch_tab(unsaved, &mut Tx::default());
    assert!(rt.state.files.dirty);
    assert_eq!(rt.state.editor.document().elements().len(), 1);
}

#[test]
fn keyboard_reopening_focuses_the_existing_tab_and_escape_dismisses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Existing.rusty-pencil");
    scene(&path);
    let mut rt = app();
    rt.state.open_path(&path, &mut Tx::default());
    let id = rt.state.tabs.active;
    rt.state.new_tab(&mut Tx::default());
    let tabs = rt.state.tabs.entries.len();
    type_key(&mut rt, "v");
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    key(&mut rt, Key::Down);
    key(&mut rt, Key::Enter);
    assert_eq!(rt.state.tabs.active, id);
    assert_eq!(rt.state.tabs.entries.len(), tabs);
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    key(&mut rt, Key::Escape);
    assert!(rt.state.menu.is_none());
}

#[test]
fn clear_history_keeps_files_and_tabs_and_persists_the_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Keep me.rusty-pencil");
    scene(&path);
    let mut rt = app();
    rt.state.tabs.workspace = Some(dir.path().join("workspace.json"));
    rt.state.open_path(&path, &mut Tx::default());
    let tabs = rt.state.tabs.entries.len();
    type_key(&mut rt, "v");
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    click(&mut rt, "clear-recent");
    assert!(rt.find("recent-empty").is_some());
    assert!(path.exists());
    assert_eq!(rt.state.tabs.entries.len(), tabs);
    let mut restored = State::new(Camera::default());
    restored.tabs.workspace = rt.state.tabs.workspace.clone();
    restored.restore_recent();
    assert!(restored.recent.paths.is_empty());
}

#[test]
fn missing_recent_file_keeps_the_canvas_and_history_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Moved away.rusty-pencil");
    let mut rt = app();
    rt.state.remember_recent(&path, &rt.state.snapshot());
    type_key(&mut rt, "r");
    drag(&mut rt, (500.0, 300.0), (600.0, 400.0));
    let tab = rt.state.tabs.active;
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    click(&mut rt, "recent-file-0");
    assert_eq!(rt.state.tabs.active, tab);
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    assert_eq!(rt.state.recent.paths, [path]);
    assert!(rt.elements().iter().any(|e| {
        e.text
            .as_deref()
            .is_some_and(|t| t.contains("Could not open"))
    }));
}

#[test]
fn recent_menu_returns_to_file_and_dismissal_does_not_draw() {
    let mut rt = app();
    type_key(&mut rt, "r");
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    assert!(rt.find("recent-empty").is_some());
    key(&mut rt, Key::Left);
    assert!(rt.find("open-file").is_some());
    click(&mut rt, "open-recent");
    drag(&mut rt, (700.0, 300.0), (800.0, 400.0));
    assert!(rt.state.menu.is_none());
    assert!(rt.state.editor.document().elements().is_empty());
}

#[test]
fn corrupt_history_is_nonfatal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("recent-files.json"), b"broken").unwrap();
    let mut s = State::new(Camera::default());
    s.tabs.workspace = Some(dir.path().join("workspace.json"));
    s.restore_recent();
    assert!(s.recent.paths.is_empty());
}

#[test]
fn a_full_history_scrolls_and_keeps_the_footer_inside_a_small_window() {
    let size = (420.0, 400.0);
    let camera = Camera::centred_on(
        pen_core::WorldPt::ORIGIN,
        pen_editor::ScreenVec::new(size.0, size.1),
        1.0,
    );
    let mut state = State::new(camera);
    for at in 0..LIMIT {
        state.remember_recent(
            Path::new(&format!("/sketches/Idea-{at}.rusty-pencil")),
            &state.snapshot(),
        );
    }
    let mut rt = Runtime::with_text(
        state,
        crate::view,
        size,
        rune::Env::new(false, 1.0),
        crate::text::system(),
    );
    rt.on_key = Some(crate::on_key);
    rt.frame(0.0);
    click(&mut rt, "menu-file");
    click(&mut rt, "open-recent");
    let footer = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("clear-recent"))
        .unwrap();
    assert!(footer.rect[1] + footer.rect[3] <= size.1);
    key(&mut rt, Key::Up);
    assert_eq!(rt.state.recent.selected, Some(LIMIT - 1));
    click(&mut rt, "menu-file");
    assert!(
        rt.state.menu.is_none(),
        "the active File tab toggles the recent view closed"
    );
}
