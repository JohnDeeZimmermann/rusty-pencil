//! Text inside shapes and on arrows, through real rune input and the real font engine.
use super::*;
use pen_core::{Container, bound};

fn click(rt: &mut Runtime<State>, x: f32, y: f32) {
    drag(rt, (x, y), (x, y));
}

fn escape(rt: &mut Runtime<State>) {
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}

fn bound_text(rt: &Runtime<State>) -> pen_core::Element {
    rt.state
        .editor
        .document()
        .elements()
        .iter()
        .find(|e| e.kind.container().is_some())
        .cloned()
        .expect("bound text")
}

#[test]
fn long_text_in_a_small_box_wraps_and_grows_it_with_real_glyphs() {
    let mut rt = app();
    key(&mut rt, "r");
    drag(&mut rt, (300.0, 300.0), (460.0, 350.0));
    key(&mut rt, "v");
    click(&mut rt, 380.0, 325.0);
    click(&mut rt, 380.0, 325.0);
    assert!(
        rt.state.editor.text_session().is_some(),
        "double-click inside"
    );
    key(
        &mut rt,
        "A small box grows as you keep on typing more words",
    );
    escape(&mut rt);
    let shape = rt.state.editor.document().elements()[0].clone();
    let text = bound_text(&rt);
    assert_eq!(text.kind.container(), Some(Container::Shape(shape.id())));
    let inner = bound::text_box(&shape.kind).expect("shape");
    assert_eq!(text.kind.bounds(), inner);
    let Kind::Text {
        content,
        font,
        size,
        ..
    } = &text.kind
    else {
        panic!("text")
    };
    let (w, h) = text::measure(
        &mut text::system(),
        content,
        *font,
        *size,
        Some(inner.width()),
    );
    assert!(
        w <= inner.width() + 0.5,
        "wraps inside: {w} > {}",
        inner.width()
    );
    assert!(h > size * 1.25 * 2.0, "several lines");
    assert!(
        (inner.height() - h).abs() < 0.5,
        "the box grew to the wrapped height: {} vs {h}",
        inner.height()
    );
}

#[test]
fn an_arrow_label_follows_its_arrow_when_grabbed_and_moved() {
    let mut rt = app();
    key(&mut rt, "a");
    drag(&mut rt, (300.0, 400.0), (600.0, 400.0));
    key(&mut rt, "v");
    click(&mut rt, 450.0, 400.0);
    click(&mut rt, 450.0, 400.0);
    key(&mut rt, "yes");
    escape(&mut rt);
    let label = bound_text(&rt).kind.bounds();
    // Deselected, so the press lands on the label rather than a flat arrow's resize handle.
    click(&mut rt, 450.0, 600.0);
    drag(&mut rt, (455.0, 405.0), (455.0, 455.0));
    let moved = bound_text(&rt).kind.bounds();
    assert_eq!(
        moved,
        label.translated(0.0, 50.0),
        "grabbing the label moves its arrow"
    );
}
