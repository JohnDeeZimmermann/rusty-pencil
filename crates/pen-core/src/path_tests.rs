use crate::*;

fn element(kind: Kind) -> Element {
    Element::new(Document::new().alloc_id(), kind, Props::default())
}

#[test]
fn paths_are_relative_and_transforms_preserve_pressure() {
    let mut path = Path::from_world(&[WorldPt::new(100.0, 80.0), WorldPt::new(160.0, 120.0)]);
    path.points[1].pressure = 0.8;
    assert_eq!(path.points[1].offset, WorldPt::new(60.0, 40.0));
    let element = element(Kind::Freedraw(path));
    let moved = element.with_bounds(element.kind.bounds().translated(20.0, -10.0));
    let path = moved.kind.path().expect("path");
    assert_eq!(path.origin, WorldPt::new(120.0, 70.0));
    assert!((path.points[1].pressure - 0.8).abs() < 1e-6);
    assert_eq!(path.points[1].offset, WorldPt::new(60.0, 40.0));
}

#[test]
fn horizontal_vertical_and_dot_transforms_are_finite() {
    for end in [
        WorldPt::new(100.0, 0.0),
        WorldPt::new(0.0, 100.0),
        WorldPt::ORIGIN,
    ] {
        let shape = element(Kind::Line(Path::from_world(&[WorldPt::ORIGIN, end])));
        let changed = shape.with_bounds(WorldRect::from_corners(
            WorldPt::new(-50.0, -25.0),
            WorldPt::new(50.0, 25.0),
        ));
        assert!(
            changed
                .kind
                .path()
                .expect("path")
                .world_points()
                .all(WorldPt::is_finite)
        );
    }
}

#[test]
fn open_path_does_not_hit_closing_edge_or_hollow_interior() {
    let path = Path::from_world(&[
        WorldPt::ORIGIN,
        WorldPt::new(100.0, 0.0),
        WorldPt::new(100.0, 100.0),
    ]);
    let line = element(Kind::Line(path));
    assert!(line.hit_test(WorldPt::new(50.0, 0.0), 0.0));
    assert!(!line.hit_test(WorldPt::new(50.0, 50.0), 0.0));
    assert!(!line.hit_test(WorldPt::new(80.0, 30.0), 0.0));
}

#[test]
fn arrowhead_hits_and_ignores_repeated_endpoints() {
    let path = Path::from_world(&[
        WorldPt::ORIGIN,
        WorldPt::new(100.0, 0.0),
        WorldPt::new(100.0, 0.0),
    ]);
    let head = path.arrowhead(2.0).expect("direction");
    let arrow = element(Kind::Arrow(path));
    assert!(arrow.hit_test(head[0], 0.0));
    let empty = Path::from_world(&[WorldPt::ORIGIN, WorldPt::ORIGIN]);
    assert!(empty.arrowhead(2.0).is_none());
}

#[test]
fn path_patch_and_inverse_restore_samples() {
    let mut doc = Document::new();
    let before = Element::new(
        doc.alloc_id(),
        Kind::Arrow(Path::from_world(&[
            WorldPt::ORIGIN,
            WorldPt::new(10.0, 20.0),
        ])),
        Props::default(),
    );
    doc.apply(Command::Add {
        element: before.clone(),
        at: 0,
    })
    .expect("add");
    let after = before.with_bounds(before.kind.bounds().translated(10.0, 20.0));
    let undo = doc
        .apply(Command::Patch {
            id: before.id(),
            before: before.clone(),
            after,
        })
        .expect("patch");
    doc.apply(undo).expect("undo");
    assert_eq!(doc.elements()[0].kind, before.kind);
}
