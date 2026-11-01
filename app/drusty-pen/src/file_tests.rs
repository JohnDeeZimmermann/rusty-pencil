use super::*;
use pen_editor::ScreenVec;
#[test]
fn atomic_write_replaces_scene_without_leaving_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("canvas.drusty-pen");
    let scene = SceneFile::new(&Document::new(), AppState::default());
    write(&path, &scene).unwrap();
    write(&path, &scene).unwrap();
    assert_eq!(read(&path).unwrap(), scene);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no temporary files remain"
    );
}
#[test]
fn autosave_debounces_from_the_last_edit_and_restores_camera() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("autosave.drusty-pen");
    let mut s = State::new(Camera::default());
    s.files.recovery = Some(path.clone());
    let mut tx = Tx::default();
    s.input(InputEvent::SelectTool(pen_editor::Tool::Rectangle), &mut tx);
    s.input(
        InputEvent::PointerDown {
            at: pen_editor::ScreenPt::new(300.0, 300.0),
            mods: pen_editor::Modifiers::NONE,
        },
        &mut tx,
    );
    s.input(
        InputEvent::PointerUp {
            at: pen_editor::ScreenPt::new(450.0, 400.0),
            mods: pen_editor::Modifiers::NONE,
        },
        &mut tx,
    );
    assert!(!poll(&mut s, 0.0, &mut tx));
    s.input(
        InputEvent::Pan {
            by: ScreenVec::new(10.0, 20.0),
        },
        &mut tx,
    );
    assert!(!poll(&mut s, 0.7, &mut tx));
    assert!(!poll(&mut s, 1.1, &mut tx));
    assert!(!path.exists());
    assert!(poll(&mut s, 1.8, &mut tx));
    let scene = read(&path).unwrap();
    assert_eq!(scene.elements.len(), 1);
    let mut restored = State::new(Camera::default());
    restored.install(&scene);
    assert_eq!(
        restored.editor.document().elements(),
        s.editor.document().elements()
    );
    assert_eq!(restored.editor.camera(), s.editor.camera());
}
#[test]
fn close_flushes_pending_text_before_the_debounce_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("autosave.drusty-pen");
    let mut s = State::new(Camera::default());
    s.files.recovery = Some(path.clone());
    let mut tx = Tx::default();
    s.input(InputEvent::SelectTool(pen_editor::Tool::Text), &mut tx);
    s.input(
        InputEvent::PointerDown {
            at: pen_editor::ScreenPt::new(300.0, 300.0),
            mods: pen_editor::Modifiers::NONE,
        },
        &mut tx,
    );
    s.text_edit.insert("Recovered text");
    close(&mut s, &mut tx);
    let scene = read(&path).unwrap();
    assert!(
        matches!(&scene.elements[0].kind, pen_core::Kind::Text { content, .. } if content == "Recovered text")
    );
}

#[test]
fn recovery_includes_the_active_text_draft_without_committing_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("autosave.drusty-pen");
    let mut s = State::new(Camera::default());
    s.files.recovery = Some(path.clone());
    let mut tx = Tx::default();
    s.input(InputEvent::SelectTool(pen_editor::Tool::Text), &mut tx);
    s.input(
        InputEvent::PointerDown {
            at: pen_editor::ScreenPt::new(300.0, 300.0),
            mods: pen_editor::Modifiers::NONE,
        },
        &mut tx,
    );
    s.text_edit.insert("A draft worth keeping");
    s.sync_text(&mut tx);
    poll(&mut s, 0.0, &mut tx);
    poll(&mut s, 1.1, &mut tx);
    assert!(
        s.editor.document().elements().is_empty(),
        "autosave does not commit the edit"
    );
    let scene = read(&path).unwrap();
    assert!(
        matches!(&scene.elements[0].kind, pen_core::Kind::Text { content, .. } if content == "A draft worth keeping")
    );
}

#[test]
fn failed_autosave_does_not_spin_or_panic_and_retries_on_close() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("not-a-directory");
    std::fs::write(&blocker, b"blocker").unwrap();
    let mut s = State::new(Camera::default());
    s.files.recovery = Some(blocker.join("autosave.drusty-pen"));
    let mut tx = Tx::default();
    s.files.changed(&mut tx);
    assert!(!poll(&mut s, 0.0, &mut tx));
    assert!(poll(&mut s, 1.1, &mut tx));
    assert!(!poll(&mut s, 1.2, &mut tx));
    assert!(s.files.status.0.get().contains("Could not autosave"));
    s.files.recovery = Some(dir.path().join("good.drusty-pen"));
    close(&mut s, &mut tx);
    assert!(s.files.recovery.as_ref().unwrap().exists());
}
