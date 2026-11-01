use super::*;
use pen_core::{Document, Props, WorldRect};
fn paint(element: &Element) -> DrawList {
    let mut list = DrawList::new();
    draw_element(element, &mut list);
    list
}
#[test]
fn seeded_outlines_are_stable_and_different_seeds_differ() {
    let mut doc = Document::new();
    let kind = Kind::Ellipse(WorldRect::from_centre(WorldPt::ORIGIN, 100.0, 70.0));
    let a = Element::new(doc.alloc_id(), kind.clone(), Props::default());
    let b = Element::new(doc.alloc_id(), kind, Props::default());
    assert_eq!(paint(&a), paint(&a));
    assert_ne!(paint(&a), paint(&b));
}
#[test]
fn all_three_sloppiness_levels_differ_and_translation_preserves_noise() {
    let mut doc = Document::new();
    let mut e = Element::new(
        doc.alloc_id(),
        Kind::Rect(WorldRect::from_centre(WorldPt::ORIGIN, 80.0, 60.0)),
        Props::default(),
    );
    let artist = paint(&e);
    e.props.roughness = Roughness::Architect;
    let architect = paint(&e);
    e.props.roughness = Roughness::Cartoonist;
    let cartoonist = paint(&e);
    assert_ne!(artist, architect);
    assert_ne!(artist, cartoonist);
    let shifted = paint(&e.with_bounds(e.kind.bounds().translated(40.0, 80.0)));
    for (a, b) in cartoonist.prims().iter().zip(shifted.prims()) {
        if let (Prim::Stroke { points: a, .. }, Prim::Stroke { points: b, .. }) = (a, b) {
            assert!(
                a.iter()
                    .zip(b)
                    .all(|(a, b)| (b.x - a.x - 40.0).abs() < 0.001
                        && (b.y - a.y - 80.0).abs() < 0.001)
            );
        }
    }
}

#[test]
fn releasing_a_draft_keeps_exactly_the_same_seeded_outline() {
    use pen_editor::{Editor, InputEvent, Modifiers, ScreenPt};
    let mut editor = Editor::default();
    editor.handle(InputEvent::PointerDown {
        at: ScreenPt::new(0.0, 0.0),
        mods: Modifiers::NONE,
    });
    editor.handle(InputEvent::PointerMove {
        at: ScreenPt::new(200.0, 100.0),
        mods: Modifiers::NONE,
    });
    let draft = editor.draft().unwrap();
    let mut drawing = DrawList::new();
    draw_seeded(&draft.kind, &draft.props, draft.seed, &mut drawing);
    editor.handle(InputEvent::PointerUp {
        at: ScreenPt::new(200.0, 100.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(drawing, paint(&editor.document().elements()[0]));
}
