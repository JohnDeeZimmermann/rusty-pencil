use super::*;

fn setup() -> (Document, Element) {
    let mut doc = Document::new();
    let element = Element::new(
        doc.alloc_id(),
        Kind::Rect(rect(0.0, 0.0, 100.0, 60.0)),
        Props::default(),
    );
    doc.apply(Command::Add {
        element: element.clone(),
        at: 0,
    })
    .expect("fresh");
    (doc, element)
}
fn rect(x: f32, y: f32, w: f32, h: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x, y), WorldPt::new(x + w, y + h))
}
fn patch(before: &Element, after: &Element) -> Command {
    Command::Patch {
        id: before.id(),
        before: before.clone(),
        after: after.clone(),
    }
}

#[test]
fn patch_and_repeated_undo_redo_advance_cache_versions() {
    let (mut doc, before) = setup();
    let after = before.with_bounds(rect(20.0, 30.0, 100.0, 60.0));
    let mut inverse = doc.apply(patch(&before, &after)).expect("patch");
    for i in 0..6 {
        let expected = if i % 2 == 0 { &after } else { &before };
        let current = doc.get(before.id()).expect("present");
        assert_eq!(current.kind, expected.kind);
        assert_eq!(current.version(), i + 2);
        assert_eq!(current.seed(), before.seed());
        inverse = doc.apply(inverse).expect("inverse");
    }
}

#[test]
fn failing_nested_group_is_atomic_including_versions_and_revision() {
    let (mut doc, before) = setup();
    let after = before.with_bounds(rect(20.0, 30.0, 100.0, 60.0));
    let revision = doc.revision();
    let bad = Command::Group(vec![
        patch(&before, &after),
        Command::Group(vec![
            Command::Remove { id: before.id() },
            Command::Remove { id: before.id() },
        ]),
    ]);
    assert_eq!(doc.apply(bad), Err(DocError::Missing(before.id())));
    assert_eq!(doc.elements(), [before]);
    assert_eq!(doc.revision(), revision);
}

#[test]
fn stale_patch_and_identity_change_are_rejected() {
    let (mut doc, before) = setup();
    let after = before.with_bounds(rect(20.0, 30.0, 100.0, 60.0));
    doc.apply(patch(&before, &after)).expect("patch");
    assert_eq!(
        doc.apply(patch(&before, &after)),
        Err(DocError::Conflict(before.id()))
    );
    let wrong = Element::new(doc.alloc_id(), before.kind.clone(), before.props);
    assert_eq!(
        doc.apply(patch(&after, &wrong)),
        Err(DocError::Conflict(before.id()))
    );
}

#[test]
fn group_sequences_validate_against_previous_commands_and_reverse_z_order() {
    let (mut doc, first) = setup();
    let second = Element::new(doc.alloc_id(), first.kind.clone(), first.props);
    let moved = second.with_bounds(rect(200.0, 0.0, 100.0, 60.0));
    let inverse = doc
        .apply(Command::Group(vec![
            Command::Add {
                element: second.clone(),
                at: 0,
            },
            patch(&second, &moved),
            Command::Remove { id: first.id() },
        ]))
        .expect("valid group");
    assert_eq!(doc.elements()[0].kind, moved.kind);
    doc.apply(inverse).expect("undo group");
    assert_eq!(doc.elements(), [first]);
}
