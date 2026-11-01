use crate::bound::{
    TEXT_PADDING, bound_text, fit, follow, grown_to_fit, label_anchor, owner, text_box,
};
use crate::file::{AppState, SceneFile, load, save};
use crate::*;

fn rect(x: f32, y: f32, w: f32, h: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x, y), WorldPt::new(x + w, y + h))
}

fn text(id: ElementId, container: Option<Container>, bounds: WorldRect) -> Element {
    Element::new(
        id,
        Kind::Text {
            content: "Hi".into(),
            font: Font::HandDrawn,
            size: 20.0,
            align: TextAlign::Centre,
            bounds,
            container,
        },
        Props::default(),
    )
}

/// A rectangle with text inside it, and an arrow with a label. Ids in that order.
fn doc() -> (Document, [ElementId; 4]) {
    let mut doc = Document::new();
    let shape = Element::new(
        doc.alloc_id(),
        Kind::Rect(rect(0.0, 0.0, 100.0, 60.0)),
        Props::default(),
    );
    let inside = fit(
        &text(
            doc.alloc_id(),
            Container::for_element(&shape),
            rect(0.0, 0.0, 1.0, 1.0),
        ),
        &shape,
    );
    let arrow = Element::new(
        doc.alloc_id(),
        Kind::Arrow(Path::from_world(&[
            WorldPt::new(0.0, 200.0),
            WorldPt::new(100.0, 200.0),
        ])),
        Props::default(),
    );
    let label = fit(
        &text(
            doc.alloc_id(),
            Container::for_element(&arrow),
            rect(0.0, 0.0, 30.0, 20.0),
        ),
        &arrow,
    );
    let ids = [shape.id(), inside.id(), arrow.id(), label.id()];
    let adds = [shape, inside, arrow, label]
        .into_iter()
        .enumerate()
        .map(|(at, element)| Command::Add { element, at })
        .collect();
    doc.apply(Command::Group(adds)).expect("fresh ids");
    (doc, ids)
}

#[test]
fn text_boxes_are_inscribed_and_padded() {
    let r = rect(0.0, 0.0, 200.0, 100.0);
    assert_eq!(
        text_box(&Kind::Rect(r)),
        Some(rect(TEXT_PADDING, TEXT_PADDING, 190.0, 90.0))
    );
    let diamond = text_box(&Kind::Diamond(r)).expect("shape");
    assert!((diamond.width() - (100.0 - 2.0 * TEXT_PADDING)).abs() < 1e-3);
    assert_eq!(diamond.centre(), r.centre());
    let ellipse = text_box(&Kind::Ellipse(r)).expect("shape");
    assert!((ellipse.height() - (100.0 / 2f32.sqrt() - 2.0 * TEXT_PADDING)).abs() < 1e-3);
    assert_eq!(
        text_box(&Kind::Rect(rect(0.0, 0.0, 4.0, 4.0))).map(|b| b.width()),
        Some(0.0)
    );
    assert_eq!(
        text_box(&Kind::Line(Path::from_world(&[WorldPt::ORIGIN]))),
        None
    );
}

#[test]
fn shapes_grow_down_to_fit_text_and_never_shrink() {
    let kind = Kind::Rect(rect(10.0, 20.0, 100.0, 40.0));
    assert_eq!(grown_to_fit(&kind, 20.0), None);
    let grown = grown_to_fit(&kind, 80.0).expect("taller");
    assert_eq!(grown, rect(10.0, 20.0, 100.0, 80.0 + 2.0 * TEXT_PADDING));
    assert_eq!(grown_to_fit(&kind, f32::NAN), None);
}

#[test]
fn labels_centre_halfway_along_the_path() {
    let path = Path::from_world(&[
        WorldPt::new(0.0, 0.0),
        WorldPt::new(100.0, 0.0),
        WorldPt::new(100.0, 300.0),
    ]);
    assert_eq!(label_anchor(&path), WorldPt::new(100.0, 100.0));
    let dot = Path::from_world(&[WorldPt::new(5.0, 5.0), WorldPt::new(5.0, 5.0)]);
    assert_eq!(label_anchor(&dot), WorldPt::new(5.0, 5.0));
}

#[test]
fn moving_a_container_refits_its_text_in_one_undoable_command() {
    let (mut doc, [shape, inside, arrow, label]) = doc();
    let before = doc.get(shape).expect("shape").clone();
    let moved = before.with_bounds(rect(50.0, 50.0, 100.0, 60.0));
    let command = follow(
        &doc,
        Command::Patch {
            id: shape,
            before: before.clone(),
            after: moved.clone(),
        },
    );
    let undo = doc.apply(command).expect("valid");
    assert_eq!(
        doc.get(inside).expect("text").kind.bounds(),
        text_box(&moved.kind).expect("shape")
    );
    assert_eq!(
        doc.get(label).expect("label").kind.bounds().centre(),
        WorldPt::new(50.0, 200.0)
    );
    doc.apply(undo).expect("undo");
    assert_eq!(
        doc.get(inside).expect("text").kind.bounds(),
        text_box(&before.kind).expect("shape")
    );
    // Nothing follows an element that holds no text.
    let unrelated = Command::Remove { id: arrow };
    let followed = follow(&doc, unrelated.clone());
    assert_eq!(
        followed,
        Command::Group(vec![unrelated, Command::Remove { id: label }])
    );
}

#[test]
fn deleting_a_container_deletes_its_text_and_undo_restores_order() {
    let (mut doc, [shape, inside, ..]) = doc();
    let order: Vec<_> = doc.elements().iter().map(Element::id).collect();
    let undo = doc
        .apply(follow(&doc, Command::Remove { id: shape }))
        .expect("valid");
    assert!(doc.get(inside).is_none());
    doc.apply(undo).expect("undo");
    assert_eq!(
        doc.elements().iter().map(Element::id).collect::<Vec<_>>(),
        order
    );
}

#[test]
fn clicks_on_bound_text_act_on_the_container() {
    let (doc, [shape, inside, arrow, label]) = doc();
    assert_eq!(owner(&doc, inside), shape);
    assert_eq!(owner(&doc, label), arrow);
    assert_eq!(owner(&doc, shape), shape);
    assert_eq!(bound_text(&doc, shape).map(Element::id), Some(inside));
}

#[test]
fn a_dangling_text_binding_is_dropped_on_load_not_a_crash() {
    let (doc, [shape, inside, ..]) = doc();
    let mut scene = SceneFile::new(&doc, AppState::default());
    scene.elements.retain(|e| e.id() != shape);
    let loaded = load(&save(&scene).expect("valid scene")).expect("repaired");
    let text = loaded
        .elements
        .iter()
        .find(|e| e.id() == inside)
        .expect("kept");
    assert_eq!(text.kind.container(), None);
    // Bound text round-trips with its link.
    let full = SceneFile::new(&doc, AppState::default());
    assert_eq!(load(&save(&full).expect("valid")).expect("loads"), full);
}
