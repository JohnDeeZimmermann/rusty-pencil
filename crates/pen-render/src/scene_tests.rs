use super::*;
use pen_core::{Command, Document, Element, WorldRect};

fn document() -> Document {
    let mut doc = Document::new();
    for i in 0..3 {
        let id = doc.alloc_id();
        let kind = Kind::Rect(WorldRect::from_centre(
            WorldPt::new(i as f32 * 30.0, 0.0),
            20.0,
            20.0,
        ));
        let props = Props {
            background: Some(Srgba::rgb(0xff_0000 >> (i * 8))),
            ..Props::default()
        };
        doc.apply(Command::Add {
            element: Element::new(id, kind, props),
            at: i,
        })
        .expect("fresh");
    }
    doc
}
fn paint(elements: &[Element]) -> Vec<Prim> {
    let mut list = DrawList::new();
    for element in elements {
        draw(&element.kind, &element.props, &mut list);
    }
    list.prims().to_vec()
}
fn composed(scene: &Scene) -> Vec<Prim> {
    scene
        .parts()
        .iter()
        .flat_map(ScenePart::prims)
        .cloned()
        .collect()
}

#[test]
fn previews_replace_originals_at_their_document_z_position() {
    let doc = document();
    let mut scene = Scene::default();
    let mut expected = doc.elements().to_vec();
    expected[1] = expected[1].with_bounds(expected[1].kind.bounds().translated(15.0, 5.0));
    scene.update(&doc, &[expected[1].clone()]);
    assert_eq!(composed(&scene), paint(&expected));
    assert_eq!(scene.parts().len(), 3);
}

#[test]
fn pointer_updates_keep_static_part_revisions_and_paint() {
    let doc = document();
    let original = &doc.elements()[1];
    let mut scene = Scene::default();
    scene.update(&doc, std::slice::from_ref(original));
    let revisions: Vec<_> = scene.parts().iter().map(ScenePart::revision).collect();
    let static_paint = scene.parts()[0].prims().to_vec();
    let moved = original.with_bounds(original.kind.bounds().translated(5.0, 10.0));
    scene.update(&doc, &[moved]);
    assert_eq!(scene.parts()[0].revision(), revisions[0]);
    assert_eq!(scene.parts()[2].revision(), revisions[2]);
    assert!(scene.parts()[1].revision() > revisions[1]);
    assert_eq!(scene.parts()[0].prims(), static_paint);
}

#[test]
fn membership_changes_ignore_preview_order_and_cancellation_restores_committed_paint() {
    let doc = document();
    let mut scene = Scene::default();
    scene.update(&doc, &doc.elements()[1..2]);
    let mut expected = doc.elements().to_vec();
    expected[0] = expected[0].with_bounds(expected[0].kind.bounds().translated(10.0, 10.0));
    expected[2] = expected[2].with_bounds(expected[2].kind.bounds().translated(-10.0, -10.0));
    scene.update(&doc, &[expected[2].clone(), expected[0].clone()]);
    assert_eq!(composed(&scene), paint(&expected));
    scene.update(&doc, &[]);
    assert_eq!(composed(&scene), paint(doc.elements()));
    assert_eq!(scene.parts().len(), 1);
    let revision = scene.parts()[0].revision();
    scene.update(&doc, &[]);
    assert_eq!(scene.parts()[0].revision(), revision);
}

#[test]
fn commit_undo_and_removal_refresh_without_manual_invalidation() {
    let mut doc = document();
    let mut scene = Scene::default();
    let before = doc.elements()[1].clone();
    let after = before.with_bounds(before.kind.bounds().translated(50.0, 0.0));
    scene.update(&doc, std::slice::from_ref(&after));
    let undo = doc
        .apply(Command::Patch {
            id: before.id(),
            before: before.clone(),
            after,
        })
        .expect("patch");
    scene.update(&doc, &[]);
    assert_eq!(composed(&scene), paint(doc.elements()));
    doc.apply(undo).expect("undo");
    scene.update(&doc, &[]);
    assert_eq!(composed(&scene), paint(doc.elements()));
    doc.apply(Command::Remove { id: before.id() })
        .expect("remove");
    scene.update(&doc, &[]);
    assert_eq!(composed(&scene), paint(doc.elements()));
}
