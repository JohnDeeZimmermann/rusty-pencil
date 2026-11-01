//! Text inside shapes and on lines, driven only through input events.
use crate::*;
use pen_core::{Container, Font, Kind, TextAlign, WorldPt, WorldRect, bound};

const NONE: Modifiers = Modifiers::NONE;

fn at(x: f32, y: f32) -> ScreenPt {
    ScreenPt::new(x, y)
}

fn drag(e: &mut Editor, from: (f32, f32), to: (f32, f32)) {
    e.handle(InputEvent::PointerDown {
        at: at(from.0, from.1),
        mods: NONE,
    });
    e.handle(InputEvent::PointerMove {
        at: at(to.0, to.1),
        mods: NONE,
    });
    e.handle(InputEvent::PointerUp {
        at: at(to.0, to.1),
        mods: NONE,
    });
}

fn type_text(e: &mut Editor, content: &str, size: (f32, f32)) {
    e.handle(InputEvent::TextChanged {
        content: content.into(),
        bounds: WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(size.0, size.1)),
    });
}

/// A transparent 200 x 100 rectangle at (100, 100), with the select tool active.
fn with_rect() -> Editor {
    let mut e = Editor::default();
    e.handle(InputEvent::SelectTool(Tool::Rectangle));
    drag(&mut e, (100.0, 100.0), (300.0, 200.0));
    e.handle(InputEvent::SelectTool(Tool::Select));
    e
}

fn text_of(e: &Editor) -> &pen_core::Element {
    e.document()
        .elements()
        .iter()
        .find(|el| matches!(el.kind, Kind::Text { .. }))
        .expect("text exists")
}

#[test]
fn double_click_inside_a_transparent_shape_writes_centred_text_in_it() {
    let mut e = with_rect();
    let rect = e.document().elements()[0].clone();
    e.handle(InputEvent::DoubleClick {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    let session = e.text_session().expect("editing");
    assert_eq!(
        session.element.kind.container(),
        Some(Container::Shape(rect.id()))
    );
    assert!(matches!(
        session.element.kind,
        Kind::Text {
            align: TextAlign::Centre,
            ..
        }
    ));
    assert_eq!(
        e.selection_bounds(),
        None,
        "no frame while writing inside a shape"
    );
    type_text(&mut e, "Hello", (40.0, 25.0));
    e.handle(InputEvent::FinishText);
    let text = text_of(&e).clone();
    assert_eq!(
        text.kind.bounds(),
        bound::text_box(&rect.kind).expect("shape")
    );
    assert_eq!(
        e.document().elements()[1].id(),
        text.id(),
        "just above its container"
    );
    assert!(e.selection().contains(rect.id()) && e.selection().len() == 1);
    e.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert_eq!(
        e.document().elements().len(),
        1,
        "one undo removes the text"
    );
}

#[test]
fn typing_past_the_box_grows_the_shape_live_and_commits_both_in_one_step() {
    let mut e = with_rect();
    let rect = e.document().elements()[0].clone();
    e.handle(InputEvent::DoubleClick {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    type_text(&mut e, "a lot of words", (190.0, 200.0));
    let grown = e
        .preview()
        .iter()
        .find(|el| el.id() == rect.id())
        .expect("container previews growth");
    assert!(grown.kind.bounds().height() > 200.0);
    assert_eq!(
        e.document().elements()[0].kind,
        rect.kind,
        "document untouched while typing"
    );
    type_text(&mut e, "short", (40.0, 25.0));
    assert!(
        e.preview().iter().all(|el| el.id() != rect.id()),
        "shrinks back to the original"
    );
    type_text(&mut e, "a lot of words", (190.0, 200.0));
    e.handle(InputEvent::FinishText);
    assert!(e.document().elements()[0].kind.bounds().height() > 200.0);
    e.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert_eq!(
        e.document().elements(),
        std::slice::from_ref(&e.document().elements()[0])
    );
    assert_eq!(e.document().elements()[0].kind, rect.kind);
}

#[test]
fn moving_a_shape_moves_its_text_live_and_together_in_history() {
    let mut e = with_rect();
    e.handle(InputEvent::DoubleClick {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    type_text(&mut e, "Hi", (20.0, 25.0));
    e.handle(InputEvent::FinishText);
    let text = text_of(&e).clone();
    // Pressing on the text grabs its container.
    e.handle(InputEvent::PointerDown {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    e.handle(InputEvent::PointerMove {
        at: at(250.0, 170.0),
        mods: NONE,
    });
    let live = e
        .preview()
        .iter()
        .find(|el| el.id() == text.id())
        .expect("text follows live");
    assert_eq!(
        live.kind.bounds(),
        text.kind.bounds().translated(50.0, 20.0)
    );
    e.handle(InputEvent::PointerUp {
        at: at(250.0, 170.0),
        mods: NONE,
    });
    assert_eq!(
        text_of(&e).kind.bounds(),
        text.kind.bounds().translated(50.0, 20.0)
    );
    e.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert_eq!(text_of(&e).kind.bounds(), text.kind.bounds());
    assert_eq!(
        e.document().elements()[0].kind.bounds().min(),
        WorldPt::new(100.0, 100.0)
    );
}

#[test]
fn deleting_a_shape_deletes_its_text_and_selection_never_holds_bound_text() {
    let mut e = with_rect();
    e.handle(InputEvent::DoubleClick {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    type_text(&mut e, "Hi", (20.0, 25.0));
    e.handle(InputEvent::FinishText);
    e.handle(InputEvent::Key {
        key: Key::Char('a'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert_eq!(e.selection().len(), 1);
    drag(&mut e, (50.0, 50.0), (400.0, 400.0));
    assert_eq!(e.selection().len(), 1, "marquee takes the container only");
    e.handle(InputEvent::Key {
        key: Key::Delete,
        mods: NONE,
    });
    assert!(e.document().elements().is_empty());
    e.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert_eq!(e.document().elements().len(), 2);
}

#[test]
fn double_click_on_existing_text_in_a_shape_edits_it() {
    let mut e = with_rect();
    e.handle(InputEvent::DoubleClick {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    type_text(&mut e, "Hi", (20.0, 25.0));
    e.handle(InputEvent::FinishText);
    let id = text_of(&e).id();
    e.handle(InputEvent::DoubleClick {
        at: at(110.0, 110.0),
        mods: NONE,
    });
    assert_eq!(e.text_session().map(|s| s.element.id()), Some(id));
    type_text(&mut e, "", (1.0, 25.0));
    e.handle(InputEvent::FinishText);
    assert_eq!(e.document().elements().len(), 1, "emptied text is removed");
    assert_eq!(e.selection().len(), 1, "its container stays selected");
}

#[test]
fn arrows_take_a_centred_label_and_ctrl_double_click_edits_points() {
    let mut e = Editor::default();
    e.handle(InputEvent::SelectTool(Tool::Arrow));
    drag(&mut e, (100.0, 100.0), (300.0, 100.0));
    e.handle(InputEvent::SelectTool(Tool::Select));
    let arrow = e.document().elements()[0].id();
    e.handle(InputEvent::DoubleClick {
        at: at(150.0, 100.0),
        mods: NONE,
    });
    type_text(&mut e, "label", (40.0, 20.0));
    e.handle(InputEvent::FinishText);
    let label = text_of(&e).kind.bounds();
    assert_eq!(label.centre(), WorldPt::new(200.0, 100.0));
    assert!((label.width() - 40.0).abs() < 1e-3);
    e.handle(InputEvent::DoubleClick {
        at: at(120.0, 100.0),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert!(e.editable_points().is_some());
    // Dragging the end point re-centres the label in the same history step.
    drag(&mut e, (300.0, 100.0), (300.0, 300.0));
    let moved = text_of(&e).kind.bounds().centre();
    assert!(
        (moved.x - 200.0).abs() < 1e-3 && (moved.y - 200.0).abs() < 1e-3,
        "{moved:?}"
    );
    e.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert_eq!(text_of(&e).kind.bounds(), label);
    assert!(e.document().get(arrow).is_some());
}

#[test]
fn text_controls_style_the_text_of_a_selected_shape() {
    let mut e = with_rect();
    e.handle(InputEvent::DoubleClick {
        at: at(200.0, 150.0),
        mods: NONE,
    });
    type_text(&mut e, "Hi", (20.0, 25.0));
    e.handle(InputEvent::FinishText);
    assert_eq!(e.styled_texts().count(), 1);
    e.handle(InputEvent::TextStyle {
        font: Font::Code,
        size: 36.0,
        align: TextAlign::Right,
    });
    assert!(matches!(
        text_of(&e).kind,
        Kind::Text {
            font: Font::Code,
            align: TextAlign::Right,
            ..
        }
    ));
    e.handle(InputEvent::Key {
        key: Key::Escape,
        mods: NONE,
    });
    assert_eq!(e.styled_texts().count(), 0);
}

#[test]
fn double_click_on_empty_canvas_starts_free_text() {
    let mut e = Editor::default();
    e.handle(InputEvent::SelectTool(Tool::Select));
    e.handle(InputEvent::DoubleClick {
        at: at(40.0, 60.0),
        mods: NONE,
    });
    let session = e.text_session().expect("writing");
    assert_eq!(session.element.kind.container(), None);
    assert_eq!(
        session.element.kind.bounds().min(),
        WorldPt::new(40.0, 60.0)
    );
}
