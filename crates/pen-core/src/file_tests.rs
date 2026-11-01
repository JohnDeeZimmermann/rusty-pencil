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

#[test]
fn m5_styles_and_settings_round_trip_and_older_styles_supply_defaults() {
    use crate::{Arrowhead, FillStyle, Settings, StrokeStyle};
    let mut scene = scene();
    scene.app_state.settings = Settings {
        dark: true,
        grid: true,
        tool_lock: true,
        canvas: crate::Srgba::rgb(0xfff9db),
    };
    scene.app_state.style.fill = FillStyle::CrossHatch;
    scene.app_state.style.stroke_style = StrokeStyle::Dotted;
    scene.app_state.style.opacity = 45;
    scene.app_state.style.start_head = Arrowhead::DiamondOutline;
    scene.app_state.style.end_head = Arrowhead::Triangle;
    for element in &mut scene.elements {
        element.props = scene.app_state.style;
    }
    assert_eq!(load(&save(&scene).unwrap()).unwrap(), scene);
    let mut value = serde_json::to_value(&scene).unwrap();
    value["app_state"]
        .as_object_mut()
        .unwrap()
        .remove("settings");
    for element in value["elements"].as_array_mut().unwrap() {
        for key in ["fill", "stroke_style", "opacity", "start_head", "end_head"] {
            element["props"].as_object_mut().unwrap().remove(key);
        }
    }
    let loaded = load(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(loaded.app_state.settings, Settings::default());
    assert!(loaded.elements.iter().all(|e| e.props == Props::default()));
    scene.app_state.style.opacity = 101;
    assert!(save(&scene).is_err());
}

#[test]
fn hex_parsing_and_dark_colours_preserve_alpha_hue_and_stored_values() {
    use crate::Srgba;
    assert_eq!(Srgba::from_hex(" #aB120f "), Some(Srgba::rgb(0xab120f)));
    for invalid in ["#fff", "red", "#12345z", "#1234567", "é1234"] {
        assert_eq!(Srgba::from_hex(invalid), None);
    }
    let original = Srgba {
        a: 128,
        ..Srgba::rgb(0x1971c2)
    };
    assert_eq!(original.dark().dark(), original);
    assert_eq!(original.dark().a, 128);
    assert_eq!(original.hex(), "#1971c2");
}
