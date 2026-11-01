use crate::*;
fn frame(doc: &mut Document) -> Element {
    let id = doc.alloc_id();
    Element::new(
        id,
        Kind::Frame {
            name: "Frame".into(),
            bounds: WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(200.0, 100.0)),
        },
        Props::default(),
    )
}
#[test]
fn invalid_frame_ownership_and_rotation_are_rejected_when_loading_files() {
    let mut doc = Document::new();
    let f = frame(&mut doc);
    let id = f.id();
    doc.apply(Command::Add { element: f, at: 0 }).unwrap();
    let mut scene = file::SceneFile::new(&doc, Default::default());
    scene.elements[0].frame_id = Some(id);
    assert!(scene.document().is_err());
    scene.elements[0].frame_id = None;
    scene.elements[0].angle = 1.0;
    assert!(scene.document().is_err());
}
#[test]
fn pre_frame_files_supply_empty_ownership_and_links() {
    let mut doc = Document::new();
    let f = frame(&mut doc);
    doc.apply(Command::Add { element: f, at: 0 }).unwrap();
    let mut value = serde_json::to_value(file::SceneFile::new(&doc, Default::default())).unwrap();
    value["version"] = serde_json::json!(2);
    let e = value["elements"][0].as_object_mut().unwrap();
    e.remove("frame_id");
    e.remove("link");
    let loaded = file::load(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(loaded.elements[0].frame_id, None);
    assert_eq!(loaded.elements[0].link, None);
}

#[test]
fn commands_reject_orphaned_children_atomically() {
    let mut doc = Document::new();
    let frame = frame(&mut doc);
    let frame_id = frame.id();
    let mut child = Element::new(doc.alloc_id(), Kind::Rect(frame.bounds()), Props::default());
    child.frame_id = Some(frame_id);
    doc.apply(Command::Group(vec![
        Command::Add {
            element: child,
            at: 0,
        },
        Command::Add {
            element: frame,
            at: 1,
        },
    ]))
    .unwrap();
    let revision = doc.revision();
    assert_eq!(
        doc.apply(Command::Remove { id: frame_id }),
        Err(DocError::Frame)
    );
    assert_eq!(doc.elements().len(), 2);
    assert_eq!(doc.revision(), revision);
}
#[test]
fn imported_frames_keep_names_children_and_web_links() {
    let scene = serde_json::json!({"type":"excalidraw","elements":[{"id":"child","type":"rectangle","x":20,"y":20,"width":40,"height":40,"frameId":"frame","link":"https://example.com"},{"id":"frame","type":"frame","name":"Overview","x":0,"y":0,"width":100,"height":100}]});
    let loaded = file::load(&serde_json::to_vec(&scene).unwrap()).unwrap();
    assert_eq!(loaded.elements[0].frame_id, Some(loaded.elements[1].id()));
    assert_eq!(
        loaded.elements[0].link,
        Some(Link::Web("https://example.com".into()))
    );
    assert!(matches!(&loaded.elements[1].kind,Kind::Frame {name,..} if name=="Overview"));
}
