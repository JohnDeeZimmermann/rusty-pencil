use super::*;

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x0, y0), WorldPt::new(x1, y1))
}

fn add(doc: &mut Document, kind: Kind) -> (ElementId, Command) {
    let id = doc.alloc_id();
    let element = Element::new(id, kind, Props::default());
    let at = doc.elements().len();
    let inverse = doc.apply(Command::Add { element, at }).expect("fresh id");
    (id, inverse)
}

#[test]
fn origin_is_zero() {
    assert_eq!(WorldPt::ORIGIN, WorldPt::new(0.0, 0.0));
    assert_eq!(WorldPt::default(), WorldPt::ORIGIN);
}

#[test]
fn rects_normalise_their_corners() {
    let r = rect(10.0, 20.0, -30.0, -40.0);
    assert_eq!(r.min(), WorldPt::new(-30.0, -40.0));
    assert_eq!(r.max(), WorldPt::new(10.0, 20.0));
    assert_eq!(r, rect(-30.0, -40.0, 10.0, 20.0));
    assert!((r.width() - 40.0).abs() < 1e-6 && (r.height() - 60.0).abs() < 1e-6);
    assert_eq!(r.centre(), WorldPt::new(-10.0, -10.0));
    let c = WorldRect::from_centre(WorldPt::new(5.0, 5.0), -2.0, 3.0);
    assert_eq!(c, rect(3.0, 2.0, 7.0, 8.0));
}

#[test]
fn degenerate_rects_have_no_area() {
    assert!(rect(0.0, 0.0, 10.0, 0.0).is_degenerate());
    assert!(rect(0.0, 0.0, f32::NAN, 5.0).is_degenerate());
    assert!(!rect(0.0, 0.0, 1.0, 1.0).is_degenerate());
}

#[test]
fn ids_are_monotonic_and_seeds_differ() {
    let mut doc = Document::new();
    let (a, _) = add(&mut doc, Kind::Rect(rect(0.0, 0.0, 1.0, 1.0)));
    let (b, _) = add(&mut doc, Kind::Ellipse(rect(0.0, 0.0, 1.0, 1.0)));
    assert!(b > a);
    let seeds: Vec<u32> = doc.elements().iter().map(Element::seed).collect();
    assert_ne!(seeds[0], seeds[1]);
    // Removing never frees an id for reuse.
    doc.apply(Command::Remove { id: b }).expect("present");
    assert!(doc.alloc_id() > b);
}

#[test]
fn apply_then_inverse_restores_the_document() {
    let mut doc = Document::new();
    add(&mut doc, Kind::Rect(rect(0.0, 0.0, 10.0, 10.0)));
    let before = doc.elements().to_vec();

    let (_, undo_add) = add(&mut doc, Kind::Diamond(rect(5.0, 5.0, 20.0, 30.0)));
    assert_eq!(doc.elements().len(), 2);
    let redo_add = doc.apply(undo_add).expect("undo add");
    assert_eq!(doc.elements(), &before[..]);

    // And the inverse of the inverse re-applies the change.
    doc.apply(redo_add).expect("redo add");
    assert_eq!(doc.elements().len(), 2);
}

#[test]
fn remove_keeps_z_order_on_undo() {
    let mut doc = Document::new();
    let ids: Vec<ElementId> = (0..3)
        .map(|i| {
            let x = i as f32;
            add(&mut doc, Kind::Rect(rect(x, x, x + 1.0, x + 1.0))).0
        })
        .collect();
    let before = doc.elements().to_vec();
    let undo = doc.apply(Command::Remove { id: ids[1] }).expect("present");
    assert_eq!(doc.elements().len(), 2);
    doc.apply(undo).expect("undo remove");
    assert_eq!(doc.elements(), &before[..]);
}

#[test]
fn bad_commands_are_errors_and_change_nothing() {
    let mut doc = Document::new();
    let (id, _) = add(&mut doc, Kind::Rect(rect(0.0, 0.0, 1.0, 1.0)));
    let revision = doc.revision();
    let dup = Element::new(id, Kind::Rect(rect(0.0, 0.0, 2.0, 2.0)), Props::default());
    assert_eq!(
        doc.apply(Command::Add {
            element: dup,
            at: 0
        }),
        Err(DocError::Duplicate(id))
    );
    doc.apply(Command::Remove { id }).expect("present");
    assert_eq!(
        doc.apply(Command::Remove { id }),
        Err(DocError::Missing(id))
    );
    assert_eq!(doc.revision(), revision + 1);
}

#[test]
fn srgba_from_hex() {
    assert_eq!(
        Srgba::rgb(0x12_34_56),
        Srgba {
            r: 0x12,
            g: 0x34,
            b: 0x56,
            a: 0xff
        }
    );
}
