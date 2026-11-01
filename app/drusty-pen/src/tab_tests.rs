use super::*;
use crate::tests::{FRAME, app, drag, key};
use rune::{KeyEvent, Mods, Runtime};

fn click(rt: &mut Runtime<State>, name: &str) {
    let (x, y) = rt.find(name).unwrap_or_else(|| panic!("missing {name}"));
    drag(rt, (x, y), (x, y));
}
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
fn cycle(rt: &mut Runtime<State>, back: bool) {
    rt.key(KeyEvent {
        key: Key::Tab,
        mods: Mods {
            ctrl: true,
            shift: back,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}

#[test]
fn tabs_keep_independent_drawings_history_selection_camera_and_tool() {
    let mut rt = app();
    drag(&mut rt, (340.0, 250.0), (550.0, 400.0));
    let drawing = rt.state.editor.document().elements().to_vec();
    let selection = rt.state.editor.selection().clone();
    rt.wheel(500.0, 350.0, 20.0);
    rt.frame(FRAME);
    let camera = *rt.state.editor.camera();
    click(&mut rt, "new-tab");
    assert_eq!(rt.state.tabs.entries.len(), 2);
    assert!(rt.state.editor.document().elements().is_empty());
    assert!(rt.state.scene.is_empty());
    assert!(
        rt.find("select-tab-2").is_some(),
        "new tab mounts immediately"
    );
    key(&mut rt, "o");
    drag(&mut rt, (600.0, 300.0), (800.0, 480.0));
    cycle(&mut rt, true);
    assert_eq!(rt.state.tabs.active, TabId(1));
    assert_eq!(rt.state.editor.document().elements(), drawing);
    assert_eq!(rt.state.editor.selection(), &selection);
    assert_eq!(rt.state.editor.camera(), &camera);
    chord(&mut rt, 'z');
    assert!(rt.state.editor.document().elements().is_empty());
    cycle(&mut rt, false);
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    assert_eq!(rt.state.editor.tool(), pen_editor::Tool::Select);
    chord(&mut rt, 'z');
    assert!(rt.state.editor.document().elements().is_empty());
    chord(&mut rt, '1');
    assert_eq!(rt.state.tabs.active, TabId(1));
    chord(&mut rt, '9');
    assert_eq!(rt.state.tabs.active, TabId(2));
}

#[test]
fn switching_finishes_text_and_cancels_a_draft_without_leaking_it() {
    let mut rt = app();
    key(&mut rt, "t");
    drag(&mut rt, (420.0, 320.0), (420.0, 320.0));
    key(&mut rt, "Keep this text");
    chord(&mut rt, 't');
    assert_eq!(rt.state.tabs.entries.len(), 2);
    cycle(&mut rt, true);
    assert!(
        matches!(&rt.state.editor.document().elements()[0].kind, pen_core::Kind::Text { content, .. } if content == "Keep this text")
    );
    assert!(rt.state.editor.text_session().is_none());
    key(&mut rt, "r");
    rt.pointer_down(400.0, 300.0);
    rt.pointer_move(500.0, 400.0);
    rt.frame(FRAME);
    assert!(rt.state.editor.draft().is_some());
    cycle(&mut rt, false);
    rt.pointer_up(500.0, 400.0);
    rt.frame(FRAME);
    assert!(rt.state.editor.draft().is_none());
    assert!(rt.state.editor.document().elements().is_empty());
    cycle(&mut rt, true);
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    assert!(rt.state.editor.draft().is_none());
}

#[test]
fn opening_files_creates_tabs_and_focuses_duplicates_without_losing_edits() {
    let mut rt = app();
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.drusty-pen");
    let b = dir.path().join("b.drusty-pen");
    crate::files::write(&a, &rt.state.snapshot()).unwrap();
    crate::files::write(&b, &rt.state.snapshot()).unwrap();
    rt.drop_files(vec![a.clone(), b], None);
    rt.frame(FRAME);
    assert_eq!(rt.state.tabs.entries.len(), 3);
    chord(&mut rt, 'z');
    assert_eq!(
        rt.state.editor.document().elements().len(),
        1,
        "opening starts with clean history"
    );
    rt.drop_files(vec![a], None);
    rt.frame(FRAME);
    assert_eq!(rt.state.tabs.entries.len(), 3);
    assert_eq!(rt.state.tabs.active, TabId(2));
    rt.drop_files(vec![dir.path().join("missing.json")], None);
    rt.frame(FRAME);
    assert_eq!(rt.state.tabs.entries.len(), 3);
    chord(&mut rt, '1');
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    chord(&mut rt, 'z');
    assert!(rt.state.editor.document().elements().is_empty());
}

#[test]
fn closing_dirty_tabs_supports_cancel_discard_and_save_and_keeps_one_canvas() {
    let mut rt = app();
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    chord(&mut rt, 'w');
    assert!(rt.find("save-close-tab").is_some());
    click(&mut rt, "cancel-close-tab");
    assert_eq!(rt.state.tabs.entries.len(), 1);
    chord(&mut rt, 'w');
    click(&mut rt, "save-close-tab");
    rt.resolve_file_dialog(None).unwrap();
    rt.frame(FRAME);
    assert!(rt.state.tabs.closing.is_none());
    chord(&mut rt, 'w');
    click(&mut rt, "save-close-tab");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kept.drusty-pen");
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert_eq!(crate::files::read(&path).unwrap().elements.len(), 1);
    assert_eq!(rt.state.tabs.entries.len(), 1);
    assert!(rt.state.editor.document().elements().is_empty());
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    chord(&mut rt, 'w');
    click(&mut rt, "discard-close-tab");
    assert!(rt.state.editor.document().elements().is_empty());
    assert_eq!(rt.state.tabs.entries.len(), 1);
    let id = rt.state.tabs.active;
    chord(&mut rt, 'w');
    assert_ne!(
        rt.state.tabs.active, id,
        "clean empty tab closes without a prompt"
    );
}

#[test]
fn save_dialog_targets_its_originating_tab_after_switching() {
    let mut rt = app();
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    chord(&mut rt, 's');
    chord(&mut rt, 't');
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("first.drusty-pen");
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert_eq!(crate::files::read(&path).unwrap().elements.len(), 1);
    assert_eq!(rt.state.tabs.active, TabId(2));
    assert!(rt.state.files.path.is_none());
    cycle(&mut rt, true);
    assert_eq!(rt.state.files.path.as_ref(), Some(&path));
    assert!(!rt.state.files.dirty);
}

#[test]
fn workspace_recovers_all_tabs_camera_paths_active_tab_and_unsaved_edits() {
    let dir = tempfile::tempdir().unwrap();
    let mut rt = app();
    rt.state.tabs.workspace = Some(dir.path().join("workspace.json"));
    rt.state.files.recovery = Some(dir.path().join("autosave.drusty-pen"));
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    let first = rt.state.snapshot();
    chord(&mut rt, 't');
    key(&mut rt, "o");
    drag(&mut rt, (600.0, 300.0), (800.0, 480.0));
    rt.wheel(500.0, 350.0, 20.0);
    rt.frame(FRAME);
    let second = rt.state.snapshot();
    crate::files::close(&mut rt.state, &mut Tx::default());
    let mut restored = State::new(Camera::default());
    restored.tabs.workspace = rt.state.tabs.workspace.clone();
    assert!(restored.restore_workspace());
    assert_eq!(restored.tabs.active, TabId(2));
    assert_eq!(restored.tabs.entries.len(), 2);
    assert_eq!(restored.snapshot(), second);
    assert!(restored.files.dirty);
    restored.switch_tab(TabId(1), &mut Tx::default());
    assert_eq!(restored.snapshot(), first);
    restored.close_tab(TabId(1), &mut Tx::default());
    let mut again = State::new(Camera::default());
    again.tabs.workspace = rt.state.tabs.workspace.clone();
    assert!(again.restore_workspace());
    assert_eq!(
        again.tabs.entries.len(),
        1,
        "closed tabs stay closed after restart"
    );
    assert_eq!(again.snapshot(), second);
}

#[test]
fn many_tabs_scroll_to_the_active_tab_and_tab_chrome_never_draws() {
    let mut rt = app();
    for _ in 0..9 {
        chord(&mut rt, 't');
    }
    assert_eq!(rt.state.tabs.entries.len(), 10);
    rt.frame(0.5);
    let (x, _) = rt.find("select-tab-10").unwrap();
    assert!(x > 0.0 && x < WINDOW_WIDTH, "active tab is visible: {x}");
    drag(&mut rt, (500.0, 24.0), (700.0, 400.0));
    assert!(rt.state.editor.document().elements().is_empty());
}
const WINDOW_WIDTH: f32 = crate::WINDOW.0;

#[test]
fn equal_revisions_in_different_tabs_present_their_own_retained_text() {
    let mut rt = app();
    for label in ["First canvas", "Second canvas"] {
        key(&mut rt, "t");
        drag(&mut rt, (400.0, 300.0), (400.0, 300.0));
        key(&mut rt, label);
        rt.key(KeyEvent {
            key: Key::Escape,
            mods: Mods::default(),
        });
        rt.frame(FRAME);
        assert!(
            rt.elements()
                .iter()
                .any(|e| e.text.as_deref() == Some(label))
        );
        if label == "First canvas" {
            chord(&mut rt, 't');
        }
    }
    for (id, label, absent) in [
        ('1', "First canvas", "Second canvas"),
        ('2', "Second canvas", "First canvas"),
    ] {
        chord(&mut rt, id);
        let elements = rt.elements();
        assert!(elements.iter().any(|e| e.text.as_deref() == Some(label)));
        assert!(!elements.iter().any(|e| e.text.as_deref() == Some(absent)));
    }
}

#[test]
fn failed_close_save_keeps_the_canvas_and_duplicate_save_paths_are_rejected() {
    let mut rt = app();
    drag(&mut rt, (350.0, 260.0), (550.0, 400.0));
    chord(&mut rt, 'w');
    click(&mut rt, "save-close-tab");
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();
    rt.resolve_file_dialog(Some(blocker.join("canvas.drusty-pen")))
        .unwrap();
    rt.frame(FRAME);
    assert!(rt.state.tabs.closing.is_some());
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    click(&mut rt, "cancel-close-tab");
    let path = dir.path().join("first.drusty-pen");
    chord(&mut rt, 's');
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    chord(&mut rt, 't');
    chord(&mut rt, 's');
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert!(rt.state.files.path.is_none());
    assert_eq!(
        crate::files::read(&path).unwrap().elements.len(),
        1,
        "another tab cannot overwrite the open file"
    );
}

#[test]
fn recovery_keeps_blank_tabs_and_skips_one_broken_scene_without_losing_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let mut rt = app();
    rt.state.tabs.workspace = Some(dir.path().join("workspace.json"));
    rt.state.files.recovery = Some(dir.path().join("autosave.drusty-pen"));
    chord(&mut rt, 't');
    let mut restored = State::new(Camera::default());
    restored.tabs.workspace = rt.state.tabs.workspace.clone();
    assert!(restored.restore_workspace());
    assert_eq!(
        restored.tabs.entries.len(),
        2,
        "even never-edited canvases survive restart"
    );
    std::fs::write(dir.path().join("tab-2.drusty-pen"), b"corrupt").unwrap();
    let mut partial = State::new(Camera::default());
    partial.tabs.workspace = rt.state.tabs.workspace.clone();
    assert!(partial.restore_workspace());
    assert_eq!(partial.tabs.entries.len(), 1);
    assert_eq!(partial.tabs.active, TabId(1));
    assert!(partial.editor.document().elements().is_empty());
}
