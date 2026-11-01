use super::*;
use pen_core::{Command, Document, Edges, Element, WorldRect};

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x0, y0), WorldPt::new(x1, y1))
}

fn paint(kind: &Kind, props: Props) -> DrawList {
    let mut out = DrawList::new();
    draw(kind, &props, &mut out);
    out
}

fn stroke_points(list: &DrawList) -> &[WorldPt] {
    match list.prims() {
        [Prim::Stroke { points, closed, .. }] => {
            assert!(closed);
            points
        }
        other => panic!("expected one stroke, got {other:?}"),
    }
}

fn within(points: &[WorldPt], r: WorldRect) -> bool {
    let eps = 1e-3;
    points.iter().all(|p| {
        p.x >= r.min().x - eps
            && p.x <= r.max().x + eps
            && p.y >= r.min().y - eps
            && p.y <= r.max().y + eps
    })
}

#[test]
fn sharp_rect_is_its_four_corners() {
    let props = Props {
        edges: Edges::Sharp,
        ..Props::default()
    };
    let list = paint(&Kind::Rect(rect(0.0, 0.0, 100.0, 50.0)), props);
    assert_eq!(
        stroke_points(&list),
        &[
            WorldPt::new(0.0, 0.0),
            WorldPt::new(100.0, 0.0),
            WorldPt::new(100.0, 50.0),
            WorldPt::new(0.0, 50.0)
        ]
    );
}

#[test]
fn round_rect_stays_inside_and_cuts_corners() {
    let r = rect(0.0, 0.0, 200.0, 100.0);
    let list = paint(&Kind::Rect(r), Props::default());
    let points = stroke_points(&list);
    assert!(within(points, r));
    // The radius is 25 (a quarter of 100): the corner itself is not on the outline, the
    // tangent points are.
    assert!(!points.contains(&WorldPt::new(0.0, 0.0)));
    assert!(
        points
            .iter()
            .any(|p| (p.x - 25.0).abs() < 1e-3 && p.y.abs() < 1e-3)
    );
}

#[test]
fn every_kind_is_finite_and_inside_its_bounds() {
    let r = rect(-40.0, 10.0, 260.0, 90.0);
    for kind in [Kind::Rect(r), Kind::Diamond(r), Kind::Ellipse(r)] {
        let list = paint(&kind, Props::default());
        let points = stroke_points(&list);
        assert!(points.len() >= 4, "{kind:?}");
        assert!(points.iter().all(|p| p.is_finite()), "{kind:?}");
        assert!(within(points, r), "{kind:?}");
    }
}

#[test]
fn ellipse_is_smooth_at_max_zoom() {
    // At 3000% a 400-pt ellipse is 12 000 px across; chords must still sag < 1 px.
    let r = rect(0.0, 0.0, 400.0, 400.0);
    let list = paint(&Kind::Ellipse(r), Props::default());
    let points = stroke_points(&list);
    let c = r.centre();
    for pair in points.windows(2) {
        let mid = WorldPt::new((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
        let sag = 200.0 - (mid.x - c.x).hypot(mid.y - c.y);
        assert!(sag * 30.0 < 1.0, "sag {sag}");
    }
}

#[test]
fn background_fills_under_the_stroke() {
    let props = Props {
        background: Some(pen_core::Srgba::rgb(0xa5_d8_ff)),
        ..Props::default()
    };
    let list = paint(&Kind::Diamond(rect(0.0, 0.0, 10.0, 10.0)), props);
    assert!(matches!(
        list.prims(),
        [Prim::Fill { .. }, Prim::Stroke { .. }]
    ));
    // A degenerate draft gets no fill (it has no area to fill).
    let flat = paint(&Kind::Rect(rect(0.0, 0.0, 10.0, 0.0)), props);
    assert!(matches!(flat.prims(), [Prim::Stroke { .. }]));
}

#[test]
fn cache_rebuilds_only_new_elements_and_drops_removed_ones() {
    let mut doc = Document::new();
    let mut cache = SceneCache::new();
    let add = |doc: &mut Document, r: WorldRect| {
        let id = doc.alloc_id();
        let at = doc.elements().len();
        doc.apply(Command::Add {
            element: Element::new(id, Kind::Rect(r), Props::default()),
            at,
        })
        .expect("fresh id");
        id
    };
    let a = add(&mut doc, rect(0.0, 0.0, 10.0, 10.0));
    assert_eq!(cache.update(&doc), 1);
    add(&mut doc, rect(20.0, 0.0, 30.0, 10.0));
    assert_eq!(cache.update(&doc), 1, "only the new element is built");
    assert_eq!(cache.update(&doc), 0, "nothing changed");
    assert_eq!(cache.prims(&doc).count(), 2);

    doc.apply(Command::Remove { id: a }).expect("present");
    cache.update(&doc);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.prims(&doc).count(), 1);
}

#[test]
fn same_document_same_paint() {
    let r = rect(3.0, 4.0, 50.0, 60.0);
    for kind in [Kind::Rect(r), Kind::Diamond(r), Kind::Ellipse(r)] {
        assert_eq!(
            paint(&kind, Props::default()),
            paint(&kind, Props::default())
        );
    }
}

#[test]
fn bound_text_wraps_in_shapes_and_labels_lines() {
    let id = |n| {
        let mut doc = Document::new();
        (0..n).map(|_| doc.alloc_id()).last().expect("allocated")
    };
    let fit_of = |container| {
        let kind = Kind::Text {
            content: "Hi".into(),
            font: pen_core::Font::Normal,
            size: 20.0,
            align: pen_core::TextAlign::Centre,
            bounds: rect(0.0, 0.0, 10.0, 10.0),
            container,
        };
        match paint(&kind, Props::default()).prims() {
            [Prim::Text { fit, .. }] => *fit,
            other => panic!("expected text, got {other:?}"),
        }
    };
    assert_eq!(fit_of(None), TextFit::Free);
    assert_eq!(
        fit_of(Some(pen_core::Container::Shape(id(1)))),
        TextFit::Wrap
    );
    assert_eq!(
        fit_of(Some(pen_core::Container::Path(id(2)))),
        TextFit::Label
    );
}
