use super::*;
use crate::{Font, Path, Props, TextAlign, WorldRect};

fn scene() -> SceneFile {
    let mut doc = Document::new();
    let r = WorldRect::from_centre(WorldPt::new(5.0, 8.0), 20.0, 15.0);
    let path = Path::from_world(&[WorldPt::ORIGIN, WorldPt::new(40.0, 25.0)]);
    let kinds = [
        Kind::Rect(r),
        Kind::Diamond(r),
        Kind::Ellipse(r),
        Kind::Line(path.clone()),
        Kind::Arrow(path.clone()),
        Kind::Freedraw(path),
        Kind::Text {
            content: "Hello\n世界 👋".into(),
            font: Font::Code,
            size: 28.0,
            align: TextAlign::Centre,
            bounds: r,
            container: None,
        },
    ];
    for kind in kinds {
        let id = doc.alloc_id();
        doc.apply(Command::Add {
            element: Element::new(id, kind, Props::default()),
            at: doc.elements().len(),
        })
        .unwrap();
    }
    SceneFile::new(&doc, AppState::default())
}

#[test]
fn all_kinds_round_trip_with_identity_seed_style_and_camera() {
    let scene = scene();
    let loaded = load(&save(&scene).unwrap()).unwrap();
    assert_eq!(scene, loaded);
    let mut doc = loaded.document().unwrap();
    assert_eq!(doc.elements(), scene.elements);
    assert!(doc.alloc_id().get() > scene.elements.last().unwrap().id().get());
}

#[test]
fn garbage_truncation_and_newer_version_are_distinct() {
    assert!(matches!(load(b"garbage"), Err(FileError::Json(_))));
    assert!(matches!(load(b"{\"format\":"), Err(FileError::Truncated)));
    assert!(matches!(
        load(br#"{"format":"drusty-pen","version":999,"elements":["future kind"]}"#),
        Err(FileError::NewerVersion(999))
    ));
    assert!(matches!(
        load(br#"{"format":"other","version":1,"elements":[]}"#),
        Err(FileError::Format)
    ));
}

#[test]
fn version_zero_supplies_missing_state_and_roughness() {
    let mut value = serde_json::to_value(scene()).unwrap();
    value["version"] = 0.into();
    value.as_object_mut().unwrap().remove("app_state");
    for e in value["elements"].as_array_mut().unwrap() {
        e["props"].as_object_mut().unwrap().remove("roughness");
    }
    let loaded = load(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(loaded.version, VERSION);
    assert_eq!(loaded.app_state, AppState::default());
    assert!(
        loaded
            .elements
            .iter()
            .all(|e| e.props.roughness == crate::Roughness::Artist)
    );
}

#[test]
fn invalid_camera_duplicates_ids_and_invalid_geometry_are_rejected() {
    let mut scene = scene();
    scene.app_state.zoom = -1.0;
    assert!(matches!(save(&scene), Err(FileError::Invalid(_))));
    scene.app_state.zoom = 1.0;
    scene.elements.push(scene.elements[0].clone());
    assert!(matches!(save(&scene), Err(FileError::Invalid(_))));
    scene.elements.pop();
    scene.elements[0].props.stroke_width = f32::NAN;
    assert!(matches!(save(&scene), Err(FileError::Invalid(_))));
}
