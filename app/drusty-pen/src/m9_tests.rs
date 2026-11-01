//! M9 through Rune input, dialogs, and the same scene/export pipeline used by the window.
use super::*;
use crate::tests::{FRAME, app, drag, key};
use pen_core::Kind;
use rune::{FileDialogKind, KeyEvent, Mods};
fn click(rt: &mut rune::Runtime<State>, name: &str) {
    let (x, y) = rt.find(name).unwrap_or_else(|| panic!("missing {name}"));
    drag(rt, (x, y), (x, y));
}
fn chord(rt: &mut rune::Runtime<State>, c: char, alt: bool) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: Mods {
            cmd: !alt,
            alt,
            ..Default::default()
        },
    });
    rt.frame(FRAME);
    rt.frame(FRAME);
}
fn special(rt: &mut rune::Runtime<State>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}
fn replace(rt: &mut rune::Runtime<State>, name: &str, value: &str) {
    click(rt, name);
    chord(rt, 'a', false);
    key(rt, value);
    special(rt, Key::Enter);
    rt.frame(FRAME);
}
fn framed(rt: &mut rune::Runtime<State>) {
    key(rt, "r");
    drag(rt, (310.0, 260.0), (470.0, 350.0));
    key(rt, "f");
    drag(rt, (280.0, 230.0), (680.0, 560.0));
}
#[test]
fn frame_name_stats_and_exact_frame_export_use_real_controls() {
    let mut rt = app();
    framed(&mut rt);
    replace(&mut rt, "frame-name", "My studio");
    assert!(
        matches!(&rt.state.editor.document().elements()[1].kind,Kind::Frame {name,..} if name=="My studio")
    );
    chord(&mut rt, '/', true);
    assert!(rt.find("stat-0").is_some());
    replace(&mut rt, "stat-0", "-200");
    assert!((rt.state.editor.document().elements()[1].bounds().min().x + 200.0).abs() < 0.01);
    rt.key(KeyEvent {
        key: Key::Command('e'),
        mods: Mods {
            cmd: true,
            shift: true,
            ..Default::default()
        },
    });
    rt.frame(FRAME);
    rt.frame(FRAME);
    click(&mut rt, "export-png");
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Save);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("frame.png");
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    let image = image::open(path).unwrap();
    assert_eq!((image.width(), image.height()), (400, 330));
}
#[test]
fn library_add_drag_import_export_and_restore_are_local() {
    let mut rt = app();
    framed(&mut rt);
    let dir = tempfile::tempdir().unwrap();
    rt.state.tabs.workspace = Some(dir.path().join("workspace.json"));
    library::restore(&mut rt.state);
    click(&mut rt, "library-toggle");
    click(&mut rt, "library-add");
    assert_eq!(rt.state.library.items.len(), 1);
    assert!(rt.state.library.items[0].document().is_ok());
    let (x, y) = rt.find("library-item-0").unwrap();
    drag(&mut rt, (x, y), (500.0, 450.0));
    assert_eq!(rt.state.editor.document().elements().len(), 4);
    chord(&mut rt, 'z', false);
    assert_eq!(rt.state.editor.document().elements().len(), 2);
    click(&mut rt, "library-export");
    let path = dir.path().join("export.drusty-library");
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    click(&mut rt, "library-remove-0");
    assert!(rt.state.library.items.is_empty());
    click(&mut rt, "library-import");
    rt.resolve_file_dialog(Some(path)).unwrap();
    rt.frame(FRAME);
    assert_eq!(rt.state.library.items.len(), 1);
    rt.state.library.items.clear();
    library::restore(&mut rt.state);
    assert_eq!(rt.state.library.items.len(), 1);
}
#[test]
fn find_palette_help_links_and_view_modes_handle_focus() {
    let mut rt = app();
    key(&mut rt, "t");
    drag(&mut rt, (350.0, 280.0), (350.0, 280.0));
    key(&mut rt, "Local ideas");
    special(&mut rt, Key::Escape);
    chord(&mut rt, 'f', false);
    key(&mut rt, "ideas");
    special(&mut rt, Key::Enter);
    assert_eq!(rt.state.editor.selection().len(), 1);
    assert_eq!(rt.state.polish.modal, None);
    chord(&mut rt, 'k', false);
    replace(&mut rt, "polish-query", "https://example.com");
    click(&mut rt, "link-open");
    rt.frame(FRAME);
    assert_eq!(rt.take_url_requests(), vec!["https://example.com"]);
    chord(&mut rt, '/', false);
    key(&mut rt, "duplicate");
    special(&mut rt, Key::Enter);
    assert_eq!(rt.state.editor.document().elements().len(), 2);
    key(&mut rt, "?");
    assert!(rt.find("polish-modal").is_some());
    special(&mut rt, Key::Escape);
    chord(&mut rt, 'z', true);
    assert!(rt.find("toolbar").is_none());
    assert!(rt.find("exit-zen").is_some());
    chord(&mut rt, 'z', true);
    chord(&mut rt, 'r', true);
    let revision = rt.state.editor.document().revision();
    special(&mut rt, Key::Delete);
    key(&mut rt, "r");
    drag(&mut rt, (300.0, 400.0), (600.0, 500.0));
    assert_eq!(rt.state.editor.document().revision(), revision);
    chord(&mut rt, 'r', true);
}
#[test]
fn laser_poll_clears_trails_and_never_dirties_the_canvas() {
    let mut rt = app();
    key(&mut rt, "k");
    drag(&mut rt, (300.0, 250.0), (600.0, 450.0));
    assert!(!rt.state.editor.laser().is_empty());
    assert!(!rt.state.files.dirty);
    rt.frame(1.1);
    assert!(rt.state.editor.laser().is_empty());
    assert!(!rt.state.files.dirty);
}

#[test]
fn copying_one_frame_child_through_the_system_clipboard_keeps_a_valid_scene() {
    let mut rt = app();
    framed(&mut rt);
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    key(&mut rt, "v");
    drag(&mut rt, (310.0, 300.0), (310.0, 300.0));
    assert_eq!(rt.state.editor.selection().len(), 1);
    chord(&mut rt, 'c', false);
    chord(&mut rt, 'v', false);
    assert_eq!(rt.state.editor.document().elements().len(), 3);
}
#[test]
fn eyedropper_can_sample_an_embedded_image_pixel() {
    let mut rt = app();
    let source = include_bytes!("../../../crates/pen-core/tests/fixtures/landscape.png").to_vec();
    rt.update(|s, tx| {
        crate::images::insert(s, source, pen_editor::ScreenPt::new(500.0, 400.0), tx);
    });
    rt.frame(FRAME);
    let e = &rt.state.editor.document().elements()[0];
    let b = e.kind.bounds();
    let at = rt.state.editor.camera().world_to_screen(b.centre());
    let expected = crate::images::sample(&rt.state, at).unwrap();
    key(&mut rt, "Escape");
    key(&mut rt, "i");
    drag(&mut rt, (at.x, at.y), (at.x, at.y));
    assert_eq!(rt.state.editor.style().stroke, expected);
}
#[test]
fn ctrl_click_follows_local_element_links_without_editing_the_source() {
    let mut rt = app();
    framed(&mut rt);
    let target = rt.state.editor.document().elements()[0].id();
    rt.update(|s, tx| {
        s.input(
            InputEvent::SetLink(Some(pen_core::Link::Element(target))),
            tx,
        );
    });
    rt.frame(FRAME);
    let revision = rt.state.editor.document().revision();
    rt.physical_input(rune::PhysicalInput::Modifiers(Mods {
        cmd: true,
        ctrl: true,
        ..Default::default()
    }));
    rt.frame(FRAME);
    rt.pointer_down(500.0, 230.0);
    rt.pointer_up(500.0, 230.0);
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.document().revision(), revision);
    assert!(rt.state.editor.selection().contains(target));
}
