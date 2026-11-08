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

#[test]
fn sparse_wobble_samples_stay_within_tolerance_of_the_true_wobble() {
    // The wobble is a function of arc length, so subdividing a straight input samples the
    // true curve densely; the sparse output must pass within TOLERANCE of every such sample.
    for (length, closed) in [(6.0, false), (40.0, false), (300.0, false), (1400.0, true)] {
        let corners = if closed {
            let side = length / 4.0;
            vec![
                WorldPt::new(0.0, 0.0),
                WorldPt::new(side, 0.0),
                WorldPt::new(side, side),
                WorldPt::new(0.0, side),
            ]
        } else {
            vec![WorldPt::new(0.0, 0.0), WorldPt::new(length, 0.0)]
        };
        let sides = corners.len() - usize::from(!closed);
        let dense: Vec<_> = (0..sides)
            .flat_map(|i| {
                let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
                let n = ((b.x - a.x).hypot(b.y - a.y) / 0.05).ceil() as usize;
                (0..n).map(move |j| {
                    let t = j as f32 / n as f32;
                    WorldPt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
                })
            })
            .chain((!closed).then(|| corners[corners.len() - 1]))
            .collect();
        for seed in 0..6 {
            for amount in [0.7, 1.6] {
                let sparse = perturb(&corners, closed, seed, amount);
                let reference = perturb(&dense, closed, seed, amount);
                assert!(sparse.len() * 4 < reference.len(), "{length}: not sparse");
                let segments = sparse.len() - usize::from(!closed);
                for p in &reference {
                    let miss = (0..segments)
                        .map(|i| distance(*p, sparse[i], sparse[(i + 1) % sparse.len()]))
                        .fold(f32::INFINITY, f32::min);
                    assert!(miss <= crate::TOLERANCE, "{length}/{seed}/{amount}: {miss}");
                }
            }
        }
    }
}

fn distance(p: WorldPt, a: WorldPt, b: WorldPt) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - a.x - dx * t).hypot(p.y - a.y - dy * t)
}
