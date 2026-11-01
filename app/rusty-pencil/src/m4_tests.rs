use super::*;
use pen_core::{Font, TextAlign};
use rune::{FileDialogKind, Ime};
fn click(rt: &mut Runtime<State>, x: f32, y: f32) {
    drag(rt, (x, y), (x, y));
}
fn click_key(rt: &mut Runtime<State>, name: &str) {
    let (x, y) = rt.find(name).unwrap();
    click(rt, x, y);
}
fn special(rt: &mut Runtime<State>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}
fn command(rt: &mut Runtime<State>, c: char, shift: bool) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: Mods {
            cmd: true,
            ctrl: !cfg!(target_os = "macos"),
            shift,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}
fn begin(rt: &mut Runtime<State>) {
    key(rt, "t");
    click(rt, 350.0, 250.0);
}
fn content(rt: &Runtime<State>) -> &str {
    match &rt.state.editor.document().elements()[0].kind {
        Kind::Text { content, .. } => content,
        _ => panic!("text"),
    }
}
#[test]
fn multiline_ime_text_finishes_with_escape_and_undoes_once() {
    let mut rt = app();
    begin(&mut rt);
    assert!(rt.state.editor.text_session().is_some());
    key(&mut rt, "Hello");
    special(&mut rt, Key::Enter);
    rt.ime(Ime::Preedit("せかい".into(), None));
    rt.frame(FRAME);
    assert!(!rt.state.text_edit.text.contains("せかい"));
    rt.ime(Ime::Commit("世界".into()));
    rt.frame(FRAME);
    special(&mut rt, Key::Escape);
    assert_eq!(content(&rt), "Hello\n世界");
    command(&mut rt, 'z', false);
    assert!(kinds(&rt).is_empty());
    command(&mut rt, 'z', true);
    assert_eq!(content(&rt), "Hello\n世界");
}
#[test]
fn click_away_commits_and_double_click_edits_without_duplicate_paint() {
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "A label");
    click(&mut rt, 700.0, 550.0);
    assert_eq!(content(&rt), "A label");
    click(&mut rt, 370.0, 260.0);
    click(&mut rt, 370.0, 260.0);
    assert!(rt.state.editor.text_session().is_some());
    rt.paste("!");
    rt.frame(FRAME);
    special(&mut rt, Key::Escape);
    assert_eq!(rt.state.editor.document().elements().len(), 1);
    assert!(content(&rt).contains('!'));
    command(&mut rt, 'z', false);
    assert_eq!(content(&rt), "A label");
}
#[test]
fn property_buttons_keep_typing_focus_and_measured_geometry() {
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "One\nTwo");
    click_key(&mut rt, "font-Code");
    click_key(&mut rt, "size-L");
    click_key(&mut rt, "align-Right");
    key(&mut rt, "!");
    special(&mut rt, Key::Escape);
    let Kind::Text {
        content,
        font,
        size,
        align,
        bounds,
        ..
    } = &rt.state.editor.document().elements()[0].kind
    else {
        panic!("text")
    };
    assert_eq!(*font, Font::Code);
    assert_eq!(*align, TextAlign::Right);
    assert!((*size - 28.0).abs() < 0.01);
    assert_eq!(content, "One\nTwo!");
    let measured = text::measure(&mut text::system(), content, *font, *size, None);
    assert!((bounds.width() - measured.0).abs() < 0.01);
    command(&mut rt, '>', true);
    assert!((rt.state.editor.text_style().1 - 36.0).abs() < 0.01);
}
#[test]
fn save_open_save_as_and_drop_use_real_dialog_callbacks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("世界.rusty-pencil");
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "Saved while typing");
    command(&mut rt, 's', false);
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Save);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert_eq!(
        pen_core::file::load(&std::fs::read(&path).unwrap())
            .unwrap()
            .elements
            .len(),
        1
    );
    key(&mut rt, "r");
    drag(&mut rt, (400.0, 400.0), (550.0, 500.0));
    command(&mut rt, 's', false);
    assert!(rt.file_dialog().is_none());
    assert_eq!(
        pen_core::file::load(&std::fs::read(&path).unwrap())
            .unwrap()
            .elements
            .len(),
        2
    );
    command(&mut rt, 's', true);
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Save);
    rt.resolve_file_dialog(None).unwrap();
    rt.frame(FRAME);
    click_key(&mut rt, "menu-file");
    click_key(&mut rt, "reset-canvas");
    click_key(&mut rt, "cancel-reset");
    assert_eq!(kinds(&rt).len(), 2);
    click_key(&mut rt, "menu-file");
    click_key(&mut rt, "reset-canvas");
    click_key(&mut rt, "confirm-reset");
    assert!(kinds(&rt).is_empty());
    command(&mut rt, 'o', false);
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Open);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert_eq!(kinds(&rt).len(), 2);
    command(&mut rt, 'z', false);
    assert_eq!(kinds(&rt).len(), 2, "opening clears old undo history");
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, b"garbage").unwrap();
    rt.drop_files(vec![bad], Some((500.0, 500.0)));
    rt.frame(FRAME);
    assert_eq!(kinds(&rt).len(), 2, "failed open preserves the scene");
    rt.drop_files(vec![path], Some((500.0, 500.0)));
    rt.frame(FRAME);
    assert_eq!(content(&rt), "Saved while typing");
}
#[test]
fn text_scene_pan_and_zoom_are_live_only() {
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "Handwritten text");
    special(&mut rt, Key::Escape);
    let commits = rt.stats.commits;
    rt.wheel_event(WheelEvent::new(40.0, -20.0).at(600.0, 500.0));
    rt.frame(FRAME);
    rt.magnify_event(MagnifyEvent::new(0.5).at(600.0, 500.0));
    rt.frame(FRAME);
    assert_eq!(rt.stats.commits, commits);
}

#[test]
fn ime_preedit_grows_the_field_before_it_commits() {
    let mut rt = app();
    begin(&mut rt);
    let initial = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("canvas-text-editor"))
        .unwrap()
        .rect;
    rt.ime(Ime::Preedit("にほんごのにゅうりょくをためす".into(), None));
    rt.frame(FRAME);
    let composing = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("canvas-text-editor"))
        .unwrap()
        .rect;
    assert!(
        composing[2] > initial[2] + 100.0,
        "preedit must have space: {initial:?} -> {composing:?}"
    );
    assert!(rt.state.text_edit.text.is_empty());
    rt.ime(Ime::Commit("日本語".into()));
    rt.frame(FRAME);
    special(&mut rt, Key::Escape);
    assert_eq!(content(&rt), "日本語");
}

#[test]
fn resizing_text_preserves_its_aspect_and_a_valid_font_size() {
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "A growing label");
    special(&mut rt, Key::Escape);
    let before = rt.state.editor.document().elements()[0].kind.bounds();
    let p = rt.state.editor.camera().world_to_screen(before.max());
    drag(&mut rt, (p.x + 5.0, p.y + 5.0), (p.x + 70.0, p.y + 30.0));
    let after = rt.state.editor.document().elements()[0].kind.bounds();
    assert!((before.width() / before.height() - after.width() / after.height()).abs() < 0.01);
    assert!(after.width() > before.width());
    command(&mut rt, '>', true);
    let Kind::Text {
        content,
        font,
        size,
        bounds,
        ..
    } = &rt.state.editor.document().elements()[0].kind
    else {
        panic!("text")
    };
    let measured = text::measure(&mut text::system(), content, *font, *size, None);
    assert!((bounds.width() - measured.0).abs() < 0.01);
}

#[test]
fn selection_button_finishes_editing_existing_text_even_when_already_selected() {
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "Original");
    special(&mut rt, Key::Escape);
    click(&mut rt, 370.0, 260.0);
    click(&mut rt, 370.0, 260.0);
    assert!(rt.state.editor.text_session().is_some());
    rt.paste(" changed");
    rt.frame(FRAME);
    click_key(&mut rt, "tool-v");
    assert!(rt.state.editor.text_session().is_none());
    assert!(content(&rt).contains("changed"));
}

#[test]
fn document_text_rasterizes_at_screen_resolution_after_maximum_live_zoom() {
    let mut rt = app();
    begin(&mut rt);
    key(&mut rt, "Hello");
    special(&mut rt, Key::Escape);
    let commits = rt.stats.commits;
    rt.magnify_event(MagnifyEvent::new(29.0).at(350.0, 250.0));
    for _ in 0..30 {
        rt.frame(FRAME);
    }
    let large: Vec<_> = rt
        .instances
        .iter()
        .filter(|q| (q.params[1] - rune::render::KIND_GLYPH).abs() < 0.01 && q.rect[3] > 100.0)
        .collect();
    assert!(
        !large.is_empty(),
        "zoomed document glyphs must remain visible"
    );
    assert!(
        large.iter().all(|q| q.uv[2].abs() < 0.01),
        "glyphs must sample the atlas at 1:1 after zoom settles"
    );
    assert_eq!(
        rt.stats.commits, commits,
        "high quality text cannot rebuild the view on zoom"
    );
}

#[test]
fn text_inspector_is_contextual_and_dismisses_with_the_text_selection() {
    let mut rt = app();
    let visible = |rt: &Runtime<State>| {
        rt.elements()
            .iter()
            .any(|e| e.key.as_deref() == Some("properties"))
    };
    assert!(!visible(&rt), "an empty canvas stays clear");
    drag(&mut rt, (300.0, 300.0), (450.0, 400.0));
    key(&mut rt, "v");
    click(&mut rt, 350.0, 300.0);
    assert_eq!(rt.state.editor.selection().len(), 1);
    assert!(
        !visible(&rt),
        "shapes without text do not open the text inspector"
    );
    key(&mut rt, "t");
    assert!(
        visible(&rt),
        "the text tool exposes its defaults before placement"
    );
    click(&mut rt, 500.0, 250.0);
    key(&mut rt, "A label");
    special(&mut rt, Key::Escape);
    assert!(visible(&rt), "selected text keeps its controls");
    click(&mut rt, 800.0, 550.0);
    assert!(!visible(&rt), "clicking the canvas hides the inspector");
    click(&mut rt, 510.0, 260.0);
    assert!(visible(&rt), "selecting existing text restores it");
    key(&mut rt, "r");
    assert!(!visible(&rt), "another drawing tool hides text controls");
}

#[test]
fn outline_styles_cycle_from_the_toolbar_without_opening_a_panel() {
    let mut rt = app();
    use pen_core::Roughness;
    for expected in [
        Roughness::Cartoonist,
        Roughness::Architect,
        Roughness::Artist,
    ] {
        click_key(&mut rt, "roughness");
        assert_eq!(rt.state.editor.roughness(), expected);
        assert!(
            !rt.elements()
                .iter()
                .any(|e| e.key.as_deref() == Some("properties"))
        );
    }
    drag(&mut rt, (300.0, 300.0), (450.0, 400.0));
    key(&mut rt, "v");
    click(&mut rt, 350.0, 300.0);
    click_key(&mut rt, "roughness");
    assert_eq!(
        rt.state.editor.document().elements()[0].props.roughness,
        Roughness::Cartoonist
    );
    command(&mut rt, 'z', false);
    assert_eq!(
        rt.state.editor.document().elements()[0].props.roughness,
        Roughness::Artist
    );
}
