use crate::translation::translation;
use pen_core::{Document, Element, Font, Kind, Path, Props, TextAlign, WorldPt, WorldRect};

fn bounds() -> WorldRect {
    WorldRect::from_centre(WorldPt::ORIGIN, 50.0, 40.0)
}

#[test]
fn every_kind_reuses_only_rigid_previews() {
    let mut doc = Document::new();
    let path = Path::from_world(&[
        WorldPt::new(0.0, 0.0),
        WorldPt::new(30.0, 20.0),
        WorldPt::new(100.0, 80.0),
    ]);
    for kind in [
        Kind::Rect(bounds()),
        Kind::Diamond(bounds()),
        Kind::Ellipse(bounds()),
        Kind::Line(path.clone()),
        Kind::Arrow(path.clone()),
        Kind::Freedraw(path),
        Kind::Text {
            content: "Keep these glyphs".into(),
            font: Font::Normal,
            size: 20.0,
            align: TextAlign::Centre,
            bounds: bounds(),
            container: None,
        },
    ] {
        let before = Element::new(doc.alloc_id(), kind, Props::default());
        let moved = before.with_bounds(before.kind.bounds().translated(17.25, -31.5));
        assert_eq!(
            translation(&before, &moved),
            Some(WorldPt::new(17.25, -31.5))
        );
        let resized = before.with_bounds(WorldRect::from_centre(WorldPt::ORIGIN, 150.0, 100.0));
        assert_eq!(translation(&before, &resized), None);
        let mut rotated = moved.clone();
        rotated.angle = 0.25;
        assert_eq!(translation(&before, &rotated), None);
        let mut styled = moved;
        styled.props.opacity = 50;
        assert_eq!(translation(&before, &styled), None);
    }
}

#[test]
fn arrow_vertex_changes_invalidate_even_if_bounds_do_not_change() {
    let mut doc = Document::new();
    let before = Element::new(
        doc.alloc_id(),
        Kind::Arrow(Path::from_world(&[
            WorldPt::ORIGIN,
            WorldPt::new(50.0, 20.0),
            WorldPt::new(100.0, 80.0),
        ])),
        Props::default(),
    );
    let mut changed = before.clone();
    changed.kind.path_mut().unwrap().points[1].offset.y += 10.0;
    assert_eq!(changed.kind.bounds(), before.kind.bounds());
    assert_eq!(translation(&before, &changed), None);
}
