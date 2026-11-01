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

#[test]
fn binding_and_arrow_type_controls_work_through_rune_input() {
    let mut rt = app();
    key(&mut rt, "r");
    drag(&mut rt, (300.0, 200.0), (480.0, 300.0));
    key(&mut rt, "a");
    let (x, y) = rt.find("arrow-type-Elbow").expect("arrow type inspector");
    click(&mut rt, x, y);
    drag(&mut rt, (488.0, 250.0), (700.0, 430.0));
    let arrow = rt.state.editor.document().elements()[1].clone();
    assert_eq!(arrow.props.arrow_type, pen_core::ArrowType::Elbow);
    assert!(arrow.bindings[0].is_some());
    key(&mut rt, "v");
    drag(&mut rt, (300.0, 250.0), (350.0, 250.0));
    let after = rt.state.editor.document().get(arrow.id()).unwrap();
    assert!(
        (after.kind.path().unwrap().world_point(0).unwrap().x
            - arrow.kind.path().unwrap().world_point(0).unwrap().x
            - 50.0)
            .abs()
            < 0.01
    );
}

#[test]
fn vertical_alignment_controls_style_the_bound_text_and_the_active_edit() {
    let mut rt = app();
    key(&mut rt, "r");
    drag(&mut rt, (300.0, 200.0), (520.0, 360.0));
    key(&mut rt, "v");
    click(&mut rt, 410.0, 280.0);
    click(&mut rt, 410.0, 280.0);
    key(&mut rt, "Aligned text");
    let (x, y) = rt.find("vertical-Top").expect("vertical align inspector");
    click(&mut rt, x, y);
    assert_eq!(
        rt.state
            .editor
            .text_session()
            .unwrap()
            .element
            .props
            .vertical_align,
        pen_core::VerticalAlign::Top
    );
    escape(&mut rt);
    assert_eq!(
        rt.state.editor.vertical_align(),
        pen_core::Value::Uniform(pen_core::VerticalAlign::Top)
    );
    let (x, y) = rt.find("vertical-Bottom").unwrap();
    click(&mut rt, x, y);
    assert_eq!(
        bound_text(&rt).props.vertical_align,
        pen_core::VerticalAlign::Bottom
    );
}

#[test]
fn placed_arrow_rebinds_by_dragging_its_visible_endpoint() {
    let mut rt = app();
    key(&mut rt, "r");
    drag(&mut rt, (300.0, 200.0), (480.0, 300.0));
    key(&mut rt, "r");
    drag(&mut rt, (700.0, 400.0), (880.0, 500.0));
    key(&mut rt, "a");
    drag(&mut rt, (488.0, 250.0), (600.0, 250.0));
    let arrow = rt.state.editor.document().elements()[2].clone();
    let target = rt.state.editor.document().elements()[1].id();
    key(&mut rt, "v");
    // Select the placed arrow, then use its endpoint without entering point-edit mode.
    click(&mut rt, 550.0, 250.0);
    drag(&mut rt, (600.0, 250.0), (692.0, 450.0));
    let after = rt.state.editor.document().get(arrow.id()).unwrap();
    assert_eq!(after.bindings[1].unwrap().element, target);
    assert_eq!(after.bindings[0], arrow.bindings[0]);
}
