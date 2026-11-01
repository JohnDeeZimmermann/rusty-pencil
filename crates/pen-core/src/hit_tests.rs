use super::*;

fn bounds() -> WorldRect {
    WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(200.0, 100.0))
}
fn element(kind: Kind, filled: bool, edges: Edges) -> Element {
    let mut doc = Document::new();
    Element::new(
        doc.alloc_id(),
        kind,
        Props {
            background: filled.then_some(Srgba::INK),
            edges,
            ..Props::default()
        },
    )
}

#[test]
fn filled_shapes_hit_inside_but_hollow_shapes_only_hit_their_outline() {
    for kind in [
        Kind::Rect(bounds()),
        Kind::Diamond(bounds()),
        Kind::Ellipse(bounds()),
    ] {
        let filled = element(kind.clone(), true, Edges::Round);
        let hollow = element(kind.clone(), false, Edges::Round);
        assert!(filled.hit_test(WorldPt::new(100.0, 50.0), 5.0));
        assert!(!hollow.hit_test(WorldPt::new(100.0, 50.0), 5.0));
        assert!(hollow.hit_test(WorldPt::new(100.0, 0.0), 5.0));
        let top = outline(&kind, Edges::Round)
            .into_iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min);
        assert!(hollow.hit_test(WorldPt::new(100.0, top - 5.5), 5.0));
        assert!(!hollow.hit_test(WorldPt::new(100.0, top - 6.5), 5.0));
        assert!(!filled.hit_test(WorldPt::new(-20.0, -20.0), 5.0));
    }
}

#[test]
fn rounded_corners_and_diamond_empty_corners_follow_visible_geometry() {
    let round = element(Kind::Rect(bounds()), true, Edges::Round);
    let sharp = element(Kind::Rect(bounds()), true, Edges::Sharp);
    assert!(!round.hit_test(WorldPt::ORIGIN, 0.0));
    assert!(sharp.hit_test(WorldPt::ORIGIN, 0.0));
    for kind in [Kind::Diamond(bounds()), Kind::Ellipse(bounds())] {
        assert!(!element(kind.clone(), true, Edges::Round).hit_test(WorldPt::new(5.0, 5.0), 0.0));
    }
}

#[test]
fn tolerance_converts_from_screen_pixels_and_rejects_bad_points() {
    let element = element(Kind::Ellipse(bounds()), false, Edges::Round);
    for zoom in [0.1, 1.0, 2.0, 30.0] {
        assert!(element.hit_test(WorldPt::new(100.0, -1.0 - 4.0 / zoom), 5.0 / zoom));
        assert!(!element.hit_test(WorldPt::new(100.0, -1.0 - 7.0 / zoom), 5.0 / zoom));
    }
    assert!(!element.hit_test(WorldPt::new(f32::NAN, 0.0), 5.0));
    assert!(!element.hit_test(WorldPt::ORIGIN, f32::INFINITY));
}
