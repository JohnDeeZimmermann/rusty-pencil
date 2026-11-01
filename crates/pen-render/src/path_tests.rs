use super::*;
use pen_core::{Path, PathPoint};

#[test]
fn thousand_sample_freehand_outline_is_closed_finite_and_deterministic() {
    let input: Vec<_> = (0..1000)
        .map(|i| WorldPt::new(i as f32 * 0.5, (i as f32 * 0.03).sin() * 40.0))
        .collect();
    let mut path = Path::from_world(&input);
    for (i, p) in path.points.iter_mut().enumerate() {
        p.pressure = (i as f32 * 0.01).sin() * 0.3 + 0.5;
    }
    let points = freehand_outline(&path, 2.0);
    assert!(points.len() > 1000);
    assert_eq!(points.first(), points.last());
    assert!(points.iter().all(|p| p.is_finite()));
    assert_eq!(points, freehand_outline(&path, 2.0));
}

#[test]
fn repeated_samples_and_dots_have_finite_outlines_and_empty_paths_are_empty() {
    for input in [vec![WorldPt::ORIGIN], vec![WorldPt::ORIGIN; 1000]] {
        let outline = freehand_outline(&Path::from_world(&input), 2.0);
        assert!(outline.len() > 8);
        assert!(outline.iter().all(|p| p.is_finite()));
        assert_eq!(outline.first(), outline.last());
    }
    assert!(freehand_outline(&Path::from_world(&[]), 2.0).is_empty());
}

#[test]
fn pressure_changes_the_outline_width_and_background_does_not_fill_open_paths() {
    let mut path = Path::from_world(&[WorldPt::ORIGIN, WorldPt::new(100.0, 0.0)]);
    let height = |points: Vec<WorldPt>| {
        points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max)
            - points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min)
    };
    for p in &mut path.points {
        p.pressure = 0.1;
    }
    let thin = height(freehand_outline(&path, 2.0));
    for p in &mut path.points {
        p.pressure = 0.9;
    }
    let thick = height(freehand_outline(&path, 2.0));
    assert!(thick > thin * 1.5, "{thick} {thin}");
    let mut list = DrawList::new();
    draw(
        &Kind::Arrow(path),
        &Props {
            background: Some(Srgba::INK),
            ..Props::default()
        },
        &mut list,
    );
    assert!(matches!(
        list.prims(),
        [
            Prim::Stroke { closed: false, .. },
            Prim::Stroke { closed: false, .. }
        ]
    ));
}

#[test]
fn invalid_samples_and_widths_do_not_reach_paint() {
    let path = Path {
        origin: WorldPt::ORIGIN,
        points: vec![PathPoint {
            offset: WorldPt::new(f32::NAN, 1.0),
            pressure: 0.5,
        }],
    };
    let mut list = DrawList::new();
    for kind in [Kind::Freedraw(path.clone()), Kind::Line(path)] {
        draw(&kind, &Props::default(), &mut list);
    }
    assert!(list.is_empty());
    for width in [0.0, -1.0, f32::NAN] {
        assert!(freehand_outline(&Path::from_world(&[WorldPt::ORIGIN]), width).is_empty());
    }
}

#[test]
fn curved_arrows_keep_full_size_heads_and_hit_the_same_curve() {
    let kind = Kind::Arrow(Path::from_world(&[
        WorldPt::new(0.0, 0.0),
        WorldPt::new(100.0, 80.0),
        WorldPt::new(220.0, 0.0),
    ]));
    let props = Props {
        arrow_type: pen_core::ArrowType::Curved,
        roughness: pen_core::Roughness::Architect,
        ..Props::default()
    };
    let mut out = DrawList::new();
    draw(&kind, &props, &mut out);
    let [
        Prim::Stroke { points: curve, .. },
        Prim::Stroke { points: head, .. },
    ] = out.prims()
    else {
        panic!("curve and open head");
    };
    assert!(curve.len() > 20);
    assert!((head[0].x - head[1].x).hypot(head[0].y - head[1].y) > 15.0);
    let mut doc = pen_core::Document::new();
    let element = pen_core::Element::new(doc.alloc_id(), kind, props);
    assert!(element.hit_test(curve[curve.len() / 3], 0.1));
    assert!(element.hit_test(head[0], 0.1));
    assert!(!element.hit_test(WorldPt::new(100.0, 0.0), 0.1));
}
