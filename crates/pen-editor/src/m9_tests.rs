#![allow(clippy::float_cmp)]
use super::*;
use pen_core::{Kind, WorldPt, WorldRect};
fn point(x: f32, y: f32) -> ScreenPt {
    ScreenPt::new(x, y)
}
fn drag(ed: &mut Editor, tool: Tool, a: (f32, f32), b: (f32, f32)) {
    ed.handle(InputEvent::SelectTool(tool));
    ed.handle(InputEvent::PointerDown {
        at: point(a.0, a.1),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: point(b.0, b.1),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerUp {
        at: point(b.0, b.1),
        mods: Modifiers::NONE,
    });
}
fn key(ed: &mut Editor, c: char, ctrl: bool, alt: bool) {
    ed.handle(InputEvent::Key {
        key: Key::Char(c),
        mods: Modifiers {
            ctrl,
            alt,
            ..Default::default()
        },
    });
}
fn framed() -> Editor {
    let mut ed = Editor::default();
    drag(&mut ed, Tool::Rectangle, (40.0, 40.0), (100.0, 90.0));
    drag(&mut ed, Tool::Frame, (10.0, 10.0), (200.0, 180.0));
    ed
}
#[test]
fn frame_owns_moves_copies_and_deletes_children_in_one_undo_step() {
    let mut ed = framed();
    let child = ed.document.elements()[0].id();
    let frame = ed.document.elements()[1].id();
    assert_eq!(ed.document.get(child).unwrap().frame_id, Some(frame));
    ed.handle(InputEvent::RenameFrame("Overview".into()));
    assert!(
        matches!(&ed.document.get(frame).unwrap().kind,Kind::Frame {name,..} if name=="Overview")
    );
    ed.handle(InputEvent::Key {
        key: Key::Right,
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.document.get(child).unwrap().bounds().min().x, 41.0);
    key(&mut ed, 'z', true, false);
    assert_eq!(ed.document.get(child).unwrap().bounds().min().x, 40.0);
    key(&mut ed, 'd', true, false);
    assert_eq!(ed.document.elements().len(), 4);
    let copied_frame = ed.document.elements()[3].id();
    assert_eq!(ed.document.elements()[2].frame_id, Some(copied_frame));
    key(&mut ed, 'z', true, false);
    ed.handle(InputEvent::Key {
        key: Key::Delete,
        mods: Modifiers::NONE,
    });
    assert!(ed.document.elements().is_empty());
    key(&mut ed, 'z', true, false);
    assert_eq!(ed.document.get(child).unwrap().frame_id, Some(frame));
}
#[test]
fn dragging_children_out_and_back_updates_ownership_and_undo() {
    let mut ed = framed();
    let child = ed.document.elements()[0].id();
    let frame = ed.document.elements()[1].id();
    ed.selection = Selection::only(child);
    drag(&mut ed, Tool::Select, (60.0, 60.0), (320.0, 60.0));
    assert_eq!(ed.document.get(child).unwrap().frame_id, None);
    key(&mut ed, 'z', true, false);
    assert_eq!(ed.document.get(child).unwrap().frame_id, Some(frame));
    drag(&mut ed, Tool::Select, (60.0, 60.0), (320.0, 60.0));
    drag(&mut ed, Tool::Select, (320.0, 60.0), (80.0, 60.0));
    assert_eq!(ed.document.get(child).unwrap().frame_id, Some(frame));
}
#[test]
fn frame_preview_includes_children_and_cancel_restores_everything() {
    let mut ed = framed();
    let old = ed.document.elements().to_vec();
    ed.handle(InputEvent::PointerDown {
        at: point(150.0, 10.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: point(180.0, 30.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.preview().len(), 2);
    assert_eq!(ed.preview()[1].bounds().min(), WorldPt::new(70.0, 60.0));
    ed.handle(InputEvent::Cancel);
    assert_eq!(ed.document.elements(), old);
}
#[test]
fn geometry_links_and_frame_names_round_trip_and_undo() {
    let mut ed = framed();
    let frame = ed.document.elements()[1].id();
    let child = ed.document.elements()[0].id();
    ed.handle(InputEvent::SetLink(Some(pen_core::Link::Element(child))));
    ed.handle(InputEvent::Geometry {
        field: GeometryField::X,
        value: 80.0,
    });
    assert_eq!(ed.document.get(child).unwrap().bounds().min().x, 110.0);
    let scene = pen_core::file::SceneFile::new(&ed.document, Default::default());
    let bytes = pen_core::file::save(&scene).unwrap();
    assert_eq!(
        pen_core::file::load(&bytes).unwrap().elements,
        scene.elements
    );
    key(&mut ed, 'z', true, false);
    assert_eq!(ed.document.get(frame).unwrap().bounds().min().x, 10.0);
    ed.handle(InputEvent::Geometry {
        field: GeometryField::Angle,
        value: 90.0,
    });
    assert_eq!(ed.document.get(frame).unwrap().angle, 0.0);
}
#[test]
fn lasso_adds_selection_without_touching_history_and_escape_cancels() {
    let mut ed = Editor::default();
    drag(&mut ed, Tool::Rectangle, (20.0, 20.0), (80.0, 80.0));
    let revision = ed.document.revision();
    ed.handle(InputEvent::SelectTool(Tool::Lasso));
    ed.handle(InputEvent::PointerDown {
        at: point(0.0, 0.0),
        mods: Modifiers::NONE,
    });
    for at in [point(100.0, 0.0), point(100.0, 100.0), point(0.0, 100.0)] {
        ed.handle(InputEvent::PointerMove {
            at,
            mods: Modifiers::NONE,
        });
    }
    ed.handle(InputEvent::PointerUp {
        at: point(0.0, 0.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.selection.len(), 1);
    assert_eq!(ed.document.revision(), revision);
    ed.handle(InputEvent::SelectTool(Tool::Lasso));
    ed.handle(InputEvent::PointerDown {
        at: point(0.0, 0.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::Cancel);
    assert!(ed.lasso().is_empty());
}
#[test]
fn laser_fades_without_document_or_history_changes() {
    let mut ed = Editor::default();
    drag(&mut ed, Tool::Laser, (0.0, 0.0), (200.0, 100.0));
    assert!(!ed.laser().is_empty());
    assert!(ed.document.elements().is_empty());
    ed.handle(InputEvent::Tick(0.4));
    assert!(!ed.laser().is_empty());
    ed.handle(InputEvent::Tick(0.6));
    assert!(ed.laser().is_empty());
    assert_eq!(ed.document.revision(), 0);
}
#[test]
fn view_mode_rejects_mutations_and_allows_navigation() {
    let mut ed = framed();
    let before = ed.document.elements().to_vec();
    key(&mut ed, 'r', false, true);
    for event in [
        InputEvent::Action(Action::Delete),
        InputEvent::SelectTool(Tool::Rectangle),
        InputEvent::RenameFrame("changed".into()),
        InputEvent::Geometry {
            field: GeometryField::X,
            value: 999.0,
        },
        InputEvent::Key {
            key: Key::Char('z'),
            mods: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        },
    ] {
        ed.handle(event);
    }
    assert_eq!(ed.document.elements(), before);
    let camera = ed.camera;
    ed.handle(InputEvent::Pan {
        by: ScreenVec::new(20.0, 40.0),
    });
    assert_ne!(ed.camera, camera);
    key(&mut ed, 'r', false, true);
    assert!(!ed.modes().view);
}
#[test]
fn object_snapping_uses_edges_and_centre_and_ctrl_temporarily_disables_it() {
    let mut ed = Editor::default();
    drag(&mut ed, Tool::Rectangle, (20.0, 20.0), (100.0, 100.0));
    drag(&mut ed, Tool::Rectangle, (220.0, 180.0), (300.0, 260.0));
    key(&mut ed, 's', false, true);
    ed.handle(InputEvent::PointerDown {
        at: point(220.0, 200.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: point(102.0, 200.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.preview()[0].bounds().min().x, 100.0);
    assert!(!ed.guides().is_empty());
    ed.handle(InputEvent::Modifiers(Modifiers {
        ctrl: true,
        ..Default::default()
    }));
    assert_eq!(ed.preview()[0].bounds().min().x, 102.0);
    assert!(ed.guides().is_empty());
    ed.handle(InputEvent::Cancel);
    assert_eq!(ed.document.elements()[1].bounds().min().x, 220.0);
}
#[test]
fn placed_library_scene_gets_fresh_ids_and_preserves_frame_links() {
    let source = framed();
    let scene = pen_core::file::SceneFile::new(source.document(), Default::default());
    let mut ed = framed();
    ed.handle(InputEvent::PlaceScene {
        scene,
        at: point(500.0, 500.0),
    });
    assert_eq!(ed.document.elements().len(), 4);
    assert_eq!(
        ed.document.elements()[2].frame_id,
        Some(ed.document.elements()[3].id())
    );
    key(&mut ed, 'z', true, false);
    assert_eq!(ed.document.elements().len(), 2);
}
#[test]
fn eyedropper_samples_stored_srgb_without_document_changes() {
    let mut ed = Editor::default();
    ed.style.background = Some(pen_core::Srgba::rgb(0xcceeff));
    drag(&mut ed, Tool::Rectangle, (10.0, 10.0), (90.0, 90.0));
    ed.selection.clear();
    let revision = ed.document.revision();
    ed.handle(InputEvent::SelectTool(Tool::Eyedropper));
    ed.handle(InputEvent::PointerDown {
        at: point(50.0, 50.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.style.stroke, pen_core::Srgba::rgb(0xcceeff));
    assert_eq!(ed.document.revision(), revision);
}
#[test]
fn spatial_hit_respects_clipping_and_topmost_order() {
    let mut ed = framed();
    let child = ed.document.elements()[0].id();
    ed.selection = Selection::only(child);
    ed.handle(InputEvent::Geometry {
        field: GeometryField::Width,
        value: 300.0,
    });
    let doc = &ed.document;
    assert!(
        doc.hit(WorldPt::new(330.0, 40.0), 5.0, false, true)
            .is_none()
    );
    assert!(
        doc.hit(WorldPt::new(50.0, 40.0), 5.0, false, true)
            .is_some()
    );
    assert!(
        WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(1.0, 1.0)).contains(WorldPt::ORIGIN)
    );
}

#[test]
fn child_only_clipboard_and_live_membership_remain_valid() {
    let mut ed = framed();
    let child = ed.document.elements()[0].id();
    ed.selection = Selection::only(child);
    let copied = ed.copy_elements();
    assert_eq!(copied[0].frame_id, None);
    let mut scene = pen_core::file::SceneFile::new(ed.document(), Default::default());
    scene.elements = copied;
    assert!(scene.document().is_ok());
    ed.handle(InputEvent::PointerDown {
        at: point(60.0, 60.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: point(320.0, 60.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.preview()[0].frame_id, None);
    ed.handle(InputEvent::Cancel);
    assert!(ed.document.get(child).unwrap().frame_id.is_some());
}
#[test]
fn searching_in_view_mode_still_cannot_move_the_found_element() {
    let mut ed = framed();
    let child = ed.document.elements()[0].id();
    key(&mut ed, 'r', false, true);
    ed.handle(InputEvent::SelectElement(child));
    let original = ed.document.elements().to_vec();
    ed.handle(InputEvent::PointerDown {
        at: point(400.0, 300.0),
        mods: Modifiers::NONE,
    });
    ed.handle(InputEvent::PointerUp {
        at: point(500.0, 450.0),
        mods: Modifiers::NONE,
    });
    assert_eq!(ed.document.elements(), original);
}
#[test]
fn lasso_shift_preserves_the_previous_selection() {
    let mut ed = Editor::default();
    drag(&mut ed, Tool::Rectangle, (20.0, 20.0), (80.0, 80.0));
    drag(&mut ed, Tool::Rectangle, (200.0, 20.0), (260.0, 80.0));
    ed.handle(InputEvent::SelectTool(Tool::Lasso));
    let mods = Modifiers {
        shift: true,
        ..Default::default()
    };
    ed.handle(InputEvent::PointerDown {
        at: point(0.0, 0.0),
        mods,
    });
    for at in [point(100.0, 0.0), point(100.0, 100.0), point(0.0, 100.0)] {
        ed.handle(InputEvent::PointerMove { at, mods });
    }
    ed.handle(InputEvent::PointerUp {
        at: point(0.0, 0.0),
        mods,
    });
    assert_eq!(ed.selection.len(), 2);
}

#[test]
fn stats_width_scales_text_and_remains_undoable() {
    let mut ed = Editor::default();
    let id = ed.document.alloc_id();
    let element = Element::new(
        id,
        Kind::Text {
            content: "Text".into(),
            font: pen_core::Font::Normal,
            size: 20.0,
            align: pen_core::TextAlign::Left,
            bounds: WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(100.0, 25.0)),
            container: None,
        },
        Props::default(),
    );
    ed.document.apply(Command::Add { element, at: 0 }).unwrap();
    ed.selection = Selection::only(id);
    ed.handle(InputEvent::Geometry {
        field: GeometryField::Width,
        value: 200.0,
    });
    let e = ed.document.get(id).unwrap();
    assert_eq!(e.kind.bounds().width(), 200.0);
    assert_eq!(e.kind.bounds().height(), 50.0);
    key(&mut ed, 'z', true, false);
    assert_eq!(ed.document.get(id).unwrap().kind.bounds().width(), 100.0);
}
#[test]
fn view_mode_allows_global_appearance_changes_without_touching_content() {
    let mut ed = framed();
    let revision = ed.document.revision();
    key(&mut ed, 'r', false, true);
    let settings = pen_core::Settings {
        dark: true,
        ..ed.settings()
    };
    ed.handle(InputEvent::Settings(settings));
    assert!(ed.settings().dark);
    assert_eq!(ed.document.revision(), revision);
}
