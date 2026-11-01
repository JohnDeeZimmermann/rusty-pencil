//! Connection gestures and their atomic history, without Rune or a GPU.
use crate::*;
use pen_core::{ArrowType, Container, Property, VerticalAlign, WorldPt, WorldRect};
const NONE: Modifiers = Modifiers::NONE;
fn event(e: &mut Editor, down: bool, x: f32, y: f32, mods: Modifiers) {
    let at = ScreenPt::new(x, y);
    e.handle(if down {
        InputEvent::PointerDown { at, mods }
    } else {
        InputEvent::PointerUp { at, mods }
    });
}
fn drag(e: &mut Editor, a: (f32, f32), b: (f32, f32)) {
    event(e, true, a.0, a.1, NONE);
    event(e, false, b.0, b.1, NONE);
}
fn scene() -> Editor {
    let mut e = Editor::default();
    drag(&mut e, (100.0, 100.0), (300.0, 200.0));
    e.handle(InputEvent::SelectTool(Tool::Arrow));
    drag(&mut e, (308.0, 150.0), (500.0, 150.0));
    e
}
fn undo(e: &mut Editor) {
    e.handle(InputEvent::Key {
        key: Key::Char('z'),
        mods: Modifiers { ctrl: true, ..NONE },
    });
}
#[test]
fn bound_end_follows_a_100_point_drag_live_and_undo_restores_both() {
    let mut e = scene();
    let shape = e.document.elements()[0].clone();
    let arrow = e.document.elements()[1].clone();
    assert_eq!(arrow.bindings[0].unwrap().element, shape.id());
    assert_eq!(e.document.bound_arrows(shape.id()), &[arrow.id()]);
    // The border selects the unfilled rectangle.
    event(&mut e, true, 100.0, 150.0, NONE);
    e.handle(InputEvent::PointerMove {
        at: ScreenPt::new(200.0, 150.0),
        mods: NONE,
    });
    let live = e.preview().iter().find(|p| p.id() == arrow.id()).unwrap();
    assert_eq!(
        live.kind.path().unwrap().world_point(0),
        Some(WorldPt::new(408.0, 150.0))
    );
    assert_eq!(
        live.kind.path().unwrap().world_point(1),
        Some(WorldPt::new(500.0, 150.0))
    );
    event(&mut e, false, 200.0, 150.0, NONE);
    assert_eq!(e.document.get(arrow.id()).unwrap().kind, live_kind(408.0));
    undo(&mut e);
    assert_eq!(e.document.get(shape.id()).unwrap().kind, shape.kind);
    assert_eq!(e.document.get(arrow.id()).unwrap().kind, arrow.kind);
}
fn live_kind(x: f32) -> Kind {
    Kind::Arrow(pen_core::Path::from_world(&[
        WorldPt::new(x, 150.0),
        WorldPt::new(500.0, 150.0),
    ]))
}
#[test]
fn ctrl_changes_binding_feedback_without_pointer_motion() {
    let mut e = Editor::default();
    drag(&mut e, (100.0, 100.0), (300.0, 200.0));
    e.handle(InputEvent::SelectTool(Tool::Arrow));
    event(&mut e, true, 308.0, 150.0, NONE);
    e.handle(InputEvent::PointerMove {
        at: ScreenPt::new(500.0, 150.0),
        mods: NONE,
    });
    assert_eq!(e.binding_targets().len(), 1);
    e.handle(InputEvent::Modifiers(Modifiers { ctrl: true, ..NONE }));
    assert!(e.binding_targets().is_empty());
    event(
        &mut e,
        false,
        500.0,
        150.0,
        Modifiers { ctrl: true, ..NONE },
    );
    assert_eq!(e.document.elements()[1].bindings, [None; 2]);
}
#[test]
fn deletion_unbinds_arrows_and_undo_restores_the_links() {
    let mut e = scene();
    let shape = e.document.elements()[0].id();
    let arrow = e.document.elements()[1].id();
    e.selection = Selection::only(shape);
    e.handle(InputEvent::Action(Action::Delete));
    assert_eq!(e.document.get(arrow).unwrap().bindings, [None; 2]);
    undo(&mut e);
    assert_eq!(
        e.document.get(arrow).unwrap().bindings[0].unwrap().element,
        shape
    );
}
#[test]
fn resize_and_rotation_keep_the_same_local_attachment() {
    let mut e = scene();
    let shape = e.document.elements()[0].clone();
    let arrow = e.document.elements()[1].clone();
    for angle in [0.0, std::f32::consts::FRAC_PI_2] {
        let before = e.document.get(shape.id()).unwrap().clone();
        let mut after = before.with_bounds(WorldRect::from_corners(
            WorldPt::new(100.0, 100.0),
            WorldPt::new(400.0, 250.0),
        ));
        after.angle = angle;
        e.execute(
            Command::Patch {
                id: shape.id(),
                before,
                after: after.clone(),
            },
            e.selection.clone(),
        );
        let path = e.document.get(arrow.id()).unwrap().kind.path().unwrap();
        assert_eq!(
            path.world_point(0),
            Some(pen_core::binding::anchor(
                &after,
                arrow.bindings[0].unwrap()
            ))
        );
        assert_eq!(path.world_points().last(), Some(WorldPt::new(500.0, 150.0)));
    }
}
#[test]
fn duplicate_remaps_connections_and_arrow_only_copy_drops_them() {
    let mut e = scene();
    e.selection = Selection::all(&e.document);
    e.handle(InputEvent::Action(Action::Duplicate));
    let pasted: Vec<_> = e.selection.elements(&e.document).collect();
    assert_eq!(pasted[1].bindings[0].unwrap().element, pasted[0].id());
    let arrow = pasted[1].id();
    e.selection = Selection::only(arrow);
    e.handle(InputEvent::Action(Action::Duplicate));
    assert_eq!(
        e.selection.elements(&e.document).next().unwrap().bindings,
        [None; 2]
    );
}
#[test]
fn wrap_action_and_vertical_alignment_are_undoable() {
    let mut e = Editor::default();
    e.handle(InputEvent::SelectTool(Tool::Text));
    event(&mut e, true, 200.0, 200.0, NONE);
    e.handle(InputEvent::TextChanged {
        content: "Decision".into(),
        bounds: WorldRect::from_centre(WorldPt::new(250.0, 212.5), 50.0, 12.5),
    });
    e.handle(InputEvent::FinishText);
    let text = e.document.elements()[0].id();
    assert!(Action::WrapText.enabled(&e));
    e.handle(InputEvent::Action(Action::WrapText));
    let shape = e.selection.ids().next().unwrap();
    assert_eq!(
        e.document.get(text).unwrap().kind.container(),
        Some(Container::Shape(shape))
    );
    e.handle(InputEvent::Property(Property::VerticalAlign(
        VerticalAlign::Top,
    )));
    assert_eq!(
        e.document.get(text).unwrap().props.vertical_align,
        VerticalAlign::Top
    );
    undo(&mut e);
    assert_eq!(
        e.document.get(text).unwrap().props.vertical_align,
        VerticalAlign::Middle
    );
    undo(&mut e);
    assert_eq!(e.document.elements().len(), 1);
    assert_eq!(e.document.get(text).unwrap().kind.container(), None);
}
#[test]
fn elbow_routes_around_both_endpoint_shapes_and_follows_them() {
    let mut e = scene();
    e.handle(InputEvent::SelectTool(Tool::Rectangle));
    drag(&mut e, (450.0, 300.0), (650.0, 400.0));
    e.handle(InputEvent::SelectTool(Tool::Arrow));
    e.handle(InputEvent::Property(Property::ArrowType(ArrowType::Elbow)));
    drag(&mut e, (308.0, 150.0), (442.0, 350.0));
    let arrow = e.document.elements().last().unwrap();
    assert!(arrow.bindings.iter().all(Option::is_some));
    let points: Vec<_> = arrow.kind.path().unwrap().world_points().collect();
    assert!(points.len() >= 3);
    assert!(
        points
            .windows(2)
            .all(|s| (s[0].x - s[1].x).abs() < 0.001 || (s[0].y - s[1].y).abs() < 0.001)
    );
}

#[test]
fn endpoint_edit_binds_live_and_ctrl_unbinds_only_the_dragged_end() {
    let mut e = scene();
    let arrow = e.document.elements()[1].id();
    e.handle(InputEvent::SelectTool(Tool::Rectangle));
    drag(&mut e, (600.0, 200.0), (800.0, 300.0));
    let target = e.document.elements().last().unwrap().id();
    e.selection = Selection::only(arrow);
    e.handle(InputEvent::Key {
        key: Key::Enter,
        mods: Modifiers { ctrl: true, ..NONE },
    });
    event(&mut e, true, 500.0, 150.0, NONE);
    e.handle(InputEvent::PointerMove {
        at: ScreenPt::new(592.0, 250.0),
        mods: NONE,
    });
    assert_eq!(e.preview()[0].bindings[1].unwrap().element, target);
    assert_eq!(e.binding_targets().len(), 2);
    e.handle(InputEvent::Modifiers(Modifiers { ctrl: true, ..NONE }));
    assert!(e.preview()[0].bindings[1].is_none());
    assert!(e.preview()[0].bindings[0].is_some());
    event(&mut e, false, 592.0, 250.0, NONE);
    assert_eq!(
        e.document.get(arrow).unwrap().bindings[1].unwrap().element,
        target
    );
    undo(&mut e);
    assert_eq!(e.document.get(arrow).unwrap().bindings[1], None);
}

#[test]
fn bound_arrow_label_follows_the_shape_through_both_dependency_levels() {
    let mut e = scene();
    e.handle(InputEvent::DoubleClick {
        at: ScreenPt::new(400.0, 150.0),
        mods: NONE,
    });
    e.handle(InputEvent::TextChanged {
        content: "commit".into(),
        bounds: WorldRect::from_centre(WorldPt::ORIGIN, 20.0, 12.5),
    });
    e.handle(InputEvent::FinishText);
    let label = e.document.elements().last().unwrap().clone();
    event(&mut e, true, 100.0, 150.0, NONE);
    e.handle(InputEvent::PointerMove {
        at: ScreenPt::new(200.0, 150.0),
        mods: NONE,
    });
    let live = e
        .preview()
        .iter()
        .find(|p| p.id() == label.id())
        .unwrap()
        .kind
        .bounds();
    assert_eq!(live.centre(), WorldPt::new(454.0, 150.0));
    event(&mut e, false, 200.0, 150.0, NONE);
    assert_eq!(e.document.get(label.id()).unwrap().kind.bounds(), live);
    undo(&mut e);
    assert_eq!(e.document.get(label.id()).unwrap().kind, label.kind);
}
