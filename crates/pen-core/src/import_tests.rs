use crate::*;
#[test]
fn every_excalidraw_type_imports_and_unsupported_types_are_counted() {
    let scene = file::load(include_bytes!("../tests/fixtures/all-elements.excalidraw")).unwrap();
    assert_eq!(scene.elements.len(), 9);
    assert_eq!(scene.skipped_elements, 3);
    assert_eq!(scene.files.len(), 1);
    let doc = scene.document().unwrap();
    let saved = file::save(&file::SceneFile::new(&doc, scene.app_state)).unwrap();
    let loaded = file::load(&saved).unwrap();
    assert_eq!(loaded.files, scene.files);
    assert_eq!(loaded.elements, scene.elements);
    assert!(matches!(loaded.elements[7].kind, Kind::Image { .. }));
}
#[test]
fn invalid_image_payload_or_crop_is_rejected_without_panicking() {
    let mut scene =
        file::load(include_bytes!("../tests/fixtures/all-elements.excalidraw")).unwrap();
    let file = scene.files.values_mut().next().unwrap();
    file.width = u32::MAX;
    assert!(file::save(&scene).is_err());
    assert!(BinaryFile::from_bytes(vec![0; 256]).is_none());
}

#[test]
fn import_preserves_groups_pressure_heads_and_supported_relationships() {
    let scene = serde_json::json!({"type":"excalidraw","elements":[
        {"id":"box","type":"rectangle","x":10,"y":20,"width":120,"height":80,"groupIds":["g"],"seed":123},
        {"id":"label","type":"text","x":20,"y":40,"width":80,"height":30,"text":"Label","containerId":"box","groupIds":["g"]},
        {"id":"arrow","type":"arrow","x":130,"y":60,"width":100,"height":1,"points":[[0,0],[100,0]],"startBinding":{"elementId":"box","gap":8},"startArrowhead":"bar","endArrowhead":"triangle"},
        {"id":"pen","type":"freedraw","points":[[0,0],[30,20]],"pressures":[0.2,0.9]}
    ]});
    let loaded = file::load(&serde_json::to_vec(&scene).unwrap()).unwrap();
    let box_element = &loaded.elements[0];
    assert_eq!(box_element.seed(), 123);
    assert_eq!(box_element.group_ids, loaded.elements[1].group_ids);
    assert_eq!(
        loaded.elements[1].kind.container(),
        Some(Container::Shape(box_element.id()))
    );
    let arrow = &loaded.elements[2];
    assert_eq!(arrow.bindings[0].unwrap().element, box_element.id());
    assert_eq!(arrow.props.start_head, Arrowhead::Bar);
    assert_eq!(arrow.props.end_head, Arrowhead::Triangle);
    assert!((loaded.elements[3].kind.path().unwrap().points[1].pressure - 0.9).abs() < 0.001);
}

#[test]
fn image_commands_validate_atomically_and_saved_scenes_omit_unused_assets() {
    let mut doc = Document::new();
    let file =
        BinaryFile::from_bytes(include_bytes!("../tests/fixtures/landscape.png").to_vec()).unwrap();
    let file_id = file.id();
    let id = doc.alloc_id();
    let element = Element::new(
        id,
        Kind::Image {
            file_id: file_id.clone(),
            crop: Crop::default(),
            bounds: WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(100.0, 100.0)),
        },
        Props::default(),
    );
    assert_eq!(
        doc.apply(Command::Add {
            element: element.clone(),
            at: 0
        }),
        Err(DocError::Image)
    );
    assert_eq!(doc.revision(), 0);
    doc.apply(Command::Group(vec![
        Command::File {
            id: file_id.clone(),
            file: Some(file),
        },
        Command::Add { element, at: 0 },
    ]))
    .unwrap();
    assert_eq!(
        doc.apply(Command::File {
            id: file_id,
            file: None
        }),
        Err(DocError::Image)
    );
    assert_eq!(doc.elements().len(), 1);
    doc.apply(Command::Remove { id }).unwrap();
    assert!(
        file::SceneFile::new(&doc, Default::default())
            .files
            .is_empty()
    );
}
