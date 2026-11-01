use super::*;
use crate::{Document, Kind, Props};

#[test]
fn ninety_degree_rotation_hits_the_rotated_corner_and_updates_bounds() {
    let mut doc = Document::new();
    let mut e = Element::new(
        doc.alloc_id(),
        Kind::Rect(WorldRect::from_centre(
            WorldPt::new(100.0, 100.0),
            60.0,
            20.0,
        )),
        Props {
            edges: crate::Edges::Sharp,
            ..Props::default()
        },
    );
    e.angle = std::f32::consts::FRAC_PI_2;
    assert!(e.hit_test(WorldPt::new(120.0, 160.0), 0.1));
    assert!(!e.hit_test(WorldPt::new(160.0, 120.0), 0.1));
    assert!((e.bounds().width() - 40.0).abs() < 0.001);
    assert!((e.bounds().height() - 120.0).abs() < 0.001);
}

#[test]
fn fractional_keys_keep_room_after_repeated_insertions_at_either_edge() {
    let first = ZKey::between(None, None);
    let mut left = first.clone();
    let mut right = first.clone();
    for _ in 0..2000 {
        let a = ZKey::between(None, Some(&left));
        let b = ZKey::between(Some(&right), None);
        assert!(a < left);
        assert!(b > right);
        left = a;
        right = b;
    }
    let middle = ZKey::between(Some(&left), Some(&right));
    assert!(left < middle && middle < right);
}

#[test]
fn rotating_circles_and_diagonal_paths_has_tight_bounds() {
    let mut doc = Document::new();
    let mut circle = Element::new(
        doc.alloc_id(),
        Kind::Ellipse(WorldRect::from_centre(WorldPt::ORIGIN, 50.0, 50.0)),
        Props::default(),
    );
    circle.angle = std::f32::consts::FRAC_PI_4;
    assert!((circle.bounds().width() - 100.0).abs() < 0.001);
    let mut line = Element::new(
        doc.alloc_id(),
        Kind::Line(crate::Path::from_world(&[
            WorldPt::ORIGIN,
            WorldPt::new(100.0, 100.0),
        ])),
        Props::default(),
    );
    line.angle = -std::f32::consts::FRAC_PI_4;
    assert!(line.bounds().height() < 0.001);
    assert!((line.bounds().width() - 100.0 * 2.0_f32.sqrt()).abs() < 0.001);
}
