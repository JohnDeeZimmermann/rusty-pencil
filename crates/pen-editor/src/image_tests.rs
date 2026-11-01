use crate::*;
use pen_core::{BinaryFile, Crop, Kind, WorldPt};
fn file() -> BinaryFile {
    BinaryFile::from_bytes(include_bytes!("../../pen-core/tests/fixtures/landscape.png").to_vec())
        .unwrap()
}
fn editor() -> Editor {
    Editor::new(Camera::new(WorldPt::ORIGIN, 1.0))
}
fn undo(ed: &mut Editor) {
    ed.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        },
    });
}
#[test]
fn image_insert_crop_and_history_are_atomic_and_deduplicate_content() {
    let mut ed = editor();
    ed.handle(InputEvent::InsertImage {
        file: file(),
        at: ScreenPt::new(100.0, 100.0),
    });
    let original = ed.document().elements()[0].clone();
    assert!(original.hit_test(WorldPt::new(200.0, 180.0), 0.0));
    ed.handle(InputEvent::InsertImage {
        file: file(),
        at: ScreenPt::new(500.0, 100.0),
    });
    assert_eq!(ed.document().files().len(), 1);
    let id = ed.selection().ids().next().unwrap();
    let before = ed.document().get(id).unwrap().clone();
    ed.handle(InputEvent::CropImage(Crop {
        x: 0.25,
        width: 0.5,
        ..Crop::default()
    }));
    let cropped = ed.document().get(id).unwrap();
    assert!((cropped.kind.bounds().width() - before.kind.bounds().width() / 2.0).abs() < 0.001);
    assert!((cropped.kind.bounds().height() - before.kind.bounds().height()).abs() < 0.001);
    undo(&mut ed);
    assert_eq!(ed.document().get(id).unwrap().kind, before.kind);
    undo(&mut ed);
    assert_eq!(ed.document().elements().len(), 1);
    assert_eq!(ed.document().files().len(), 1);
    undo(&mut ed);
    assert!(ed.document().elements().is_empty());
    assert!(ed.document().files().is_empty());
    ed.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::NONE
        },
    });
    assert!(ed.selection().contains(original.id()));
    assert_eq!(ed.document().files().len(), 1);
}
#[test]
fn invalid_crop_is_ignored_and_image_moves_through_pointer_input() {
    let mut ed = editor();
    ed.handle(InputEvent::InsertImage {
        file: file(),
        at: ScreenPt::new(100.0, 100.0),
    });
    let before = ed.document().elements()[0].kind.clone();
    ed.handle(InputEvent::CropImage(Crop {
        width: f32::NAN,
        ..Crop::default()
    }));
    assert_eq!(ed.document().elements()[0].kind, before);
    ed.handle(InputEvent::PointerDown {
        at: ScreenPt::new(220.0, 180.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: ScreenPt::new(250.0, 200.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerUp {
        at: ScreenPt::new(250.0, 200.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(
        ed.document().elements()[0].kind.bounds(),
        before.bounds().translated(30.0, 20.0)
    );
    assert!(matches!(
        ed.document().elements()[0].kind,
        Kind::Image { .. }
    ));
}
