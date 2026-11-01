use crate::*;
use pen_core::{Font, Kind, Roughness, TextAlign, WorldPt, WorldRect};
fn event(editor: &mut Editor, key: char, ctrl: bool, shift: bool) {
    editor.handle(InputEvent::Key {
        key: Key::Char(key),
        mods: Modifiers {
            ctrl,
            shift,
            alt: false,
        },
    });
}
fn begin(editor: &mut Editor) {
    event(editor, 't', false, false);
    editor.handle(InputEvent::PointerDown {
        at: ScreenPt::new(40.0, 60.0),
        mods: Modifiers::NONE,
    });
}
fn write(editor: &mut Editor, content: &str) {
    editor.handle(InputEvent::TextChanged {
        content: content.into(),
        bounds: WorldRect::from_corners(WorldPt::new(40.0, 60.0), WorldPt::new(140.0, 110.0)),
    });
}
#[test]
fn text_edit_commits_as_one_history_step_and_restores_selection() {
    let mut editor = Editor::default();
    begin(&mut editor);
    write(&mut editor, "Hi");
    write(&mut editor, "Hi\n世界");
    assert!(editor.document().elements().is_empty());
    editor.handle(InputEvent::Key {
        key: Key::Escape,
        mods: Modifiers::NONE,
    });
    let elements = editor.document().elements().to_vec();
    assert_eq!(elements.len(), 1);
    event(&mut editor, 'z', true, false);
    assert!(editor.document().elements().is_empty());
    event(&mut editor, 'z', true, true);
    assert_eq!(editor.document().elements()[0].kind, elements[0].kind);
}
#[test]
fn double_click_edits_existing_text_and_empty_edit_removes_it_undoably() {
    let mut e = Editor::default();
    begin(&mut e);
    write(&mut e, "Text");
    e.handle(InputEvent::FinishText);
    e.handle(InputEvent::DoubleClick {
        at: ScreenPt::new(70.0, 80.0),
        mods: Modifiers::NONE,
    });
    assert!(e.text_session().is_some());
    assert!(matches!(&e.preview()[0].kind, Kind::Text { content, .. } if content.is_empty()));
    write(&mut e, "");
    e.handle(InputEvent::FinishText);
    assert!(e.document().elements().is_empty());
    event(&mut e, 'z', true, false);
    assert!(
        matches!(&e.document().elements()[0].kind, Kind::Text { content, .. } if content == "Text")
    );
}
#[test]
fn click_away_finishes_text_and_empty_new_draft_adds_no_history() {
    let mut e = Editor::default();
    begin(&mut e);
    e.handle(InputEvent::FinishText);
    assert!(e.document().elements().is_empty());
    begin(&mut e);
    write(&mut e, "Text");
    e.handle(InputEvent::SelectTool(Tool::Rectangle));
    assert_eq!(e.document().elements().len(), 1);
    event(&mut e, 'z', true, false);
    assert!(e.document().elements().is_empty());
}
#[test]
fn font_alignment_size_and_sloppiness_are_reversible_commands() {
    let mut e = Editor::default();
    begin(&mut e);
    write(&mut e, "Hello");
    e.handle(InputEvent::FinishText);
    e.handle(InputEvent::TextStyle {
        font: Font::Code,
        size: 28.0,
        align: TextAlign::Right,
    });
    assert!(matches!(
        e.document().elements()[0].kind,
        Kind::Text {
            font: Font::Code,
            align: TextAlign::Right,
            ..
        }
    ));
    event(&mut e, '>', true, true);
    assert!((e.text_style().1 - 36.0).abs() < 0.01);
    event(&mut e, '<', true, true);
    assert!((e.text_style().1 - 28.0).abs() < 0.01);
    e.handle(InputEvent::Roughness(Roughness::Cartoonist));
    assert_eq!(
        e.document().elements()[0].props.roughness,
        Roughness::Cartoonist
    );
    event(&mut e, 'z', true, false);
    assert_eq!(
        e.document().elements()[0].props.roughness,
        Roughness::Artist
    );
}
