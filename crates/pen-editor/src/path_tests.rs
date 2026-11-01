use super::*;
use pen_core::{Path, WorldPt};
const NONE: Modifiers = Modifiers::NONE;
const CTRL: Modifiers = Modifiers { ctrl: true, ..NONE };
const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..NONE
};
fn p(x: f32, y: f32) -> ScreenPt {
    ScreenPt::new(x, y)
}
fn key(ed: &mut Editor, key: Key, mods: Modifiers) {
    ed.handle(InputEvent::Key { key, mods });
}
fn down(ed: &mut Editor, at: ScreenPt) {
    ed.handle(InputEvent::PointerDown { at, mods: NONE });
}
fn motion(ed: &mut Editor, at: ScreenPt) {
    ed.handle(InputEvent::PointerMove { at, mods: NONE });
}
fn up(ed: &mut Editor, at: ScreenPt) {
    ed.handle(InputEvent::PointerUp { at, mods: NONE });
}
fn click(ed: &mut Editor, at: ScreenPt) {
    down(ed, at);
    up(ed, at);
}
fn editor(tool: Tool) -> Editor {
    let mut ed = Editor::default();
    ed.settings.tool_lock = true;
    ed.handle(InputEvent::SelectTool(tool));
    ed
}
fn path(ed: &Editor) -> &Path {
    ed.document().elements()[0].kind.path().expect("path")
}
fn points(ed: &Editor) -> Vec<WorldPt> {
    path(ed).world_points().collect()
}
fn draw(ed: &mut Editor) {
    down(ed, p(10.0, 10.0));
    motion(ed, p(100.0, 50.0));
    up(ed, p(100.0, 50.0));
}
fn editing() -> Editor {
    let mut ed = editor(Tool::Line);
    click(&mut ed, p(10.0, 10.0));
    click(&mut ed, p(100.0, 10.0));
    click(&mut ed, p(100.0, 100.0));
    key(&mut ed, Key::Escape, NONE);
    ed.handle(InputEvent::SelectTool(Tool::Select));
    // Plain double-click labels a line; Ctrl opens its points.
    ed.handle(InputEvent::DoubleClick {
        at: p(50.0, 10.0),
        mods: Modifiers { ctrl: true, ..NONE },
    });
    assert!(ed.editable_points().is_some());
    ed
}

#[test]
fn lines_and_arrows_drag_commit_two_endpoints_and_undo_as_one_step() {
    for tool in [Tool::Line, Tool::Arrow] {
        let mut ed = editor(tool);
        draw(&mut ed);
        assert_eq!(
            points(&ed),
            [WorldPt::new(10.0, 10.0), WorldPt::new(100.0, 50.0)]
        );
        key(&mut ed, Key::Char('z'), CTRL);
        assert!(ed.document().elements().is_empty());
        key(
            &mut ed,
            Key::Char('z'),
            Modifiers {
                shift: true,
                ..CTRL
            },
        );
        assert_eq!(path(&ed).points.len(), 2);
    }
}

#[test]
fn click_click_enter_places_three_points_including_the_live_tail() {
    let mut ed = editor(Tool::Line);
    click(&mut ed, p(10.0, 10.0));
    click(&mut ed, p(100.0, 10.0));
    motion(&mut ed, p(100.0, 100.0));
    assert!(ed.document().elements().is_empty());
    key(&mut ed, Key::Enter, NONE);
    assert_eq!(
        points(&ed),
        [
            WorldPt::new(10.0, 10.0),
            WorldPt::new(100.0, 10.0),
            WorldPt::new(100.0, 100.0)
        ]
    );
}

#[test]
fn escape_keeps_only_placed_vertices_and_focus_cancel_discards_everything() {
    let mut ed = editor(Tool::Arrow);
    click(&mut ed, p(0.0, 0.0));
    click(&mut ed, p(100.0, 0.0));
    motion(&mut ed, p(100.0, 100.0));
    key(&mut ed, Key::Escape, NONE);
    assert_eq!(points(&ed), [WorldPt::ORIGIN, WorldPt::new(100.0, 0.0)]);
    click(&mut ed, p(200.0, 200.0));
    click(&mut ed, p(300.0, 300.0));
    ed.handle(InputEvent::Cancel);
    assert_eq!(ed.document().elements().len(), 1);
    let mut ed = editor(Tool::Line);
    click(&mut ed, p(0.0, 0.0));
    key(&mut ed, Key::Escape, NONE);
    assert!(ed.document().elements().is_empty());
}

#[test]
fn double_click_finishes_without_duplicate_endpoints() {
    let mut ed = editor(Tool::Line);
    click(&mut ed, p(0.0, 0.0));
    click(&mut ed, p(100.0, 0.0));
    ed.handle(InputEvent::DoubleClick {
        at: p(100.0, 0.0),
        mods: NONE,
    });
    up(&mut ed, p(100.0, 0.0));
    assert_eq!(path(&ed).points.len(), 2);
    assert!(ed.draft().is_none());
}

#[test]
fn shift_snaps_live_tail_immediately_to_fifteen_degrees() {
    let mut ed = editor(Tool::Line);
    down(&mut ed, p(0.0, 0.0));
    motion(&mut ed, p(100.0, 35.0));
    ed.handle(InputEvent::Modifiers(SHIFT));
    let draft = ed.draft().expect("draft");
    let tip = draft
        .kind
        .path()
        .expect("path")
        .world_point(1)
        .expect("tip");
    let degrees = tip.y.atan2(tip.x).to_degrees();
    assert!((degrees - 15.0).abs() < 1e-3);
    ed.handle(InputEvent::Modifiers(NONE));
    assert_eq!(
        ed.draft()
            .expect("draft")
            .kind
            .path()
            .expect("path")
            .world_point(1),
        Some(WorldPt::new(100.0, 35.0))
    );
}

#[test]
fn backspace_removes_last_placed_vertex_and_tool_switch_discards_draft() {
    let mut ed = editor(Tool::Arrow);
    click(&mut ed, p(0.0, 0.0));
    click(&mut ed, p(100.0, 0.0));
    click(&mut ed, p(100.0, 100.0));
    key(&mut ed, Key::Backspace, NONE);
    key(&mut ed, Key::Escape, NONE);
    assert_eq!(path(&ed).points.len(), 2);
    click(&mut ed, p(200.0, 200.0));
    key(&mut ed, Key::Char('r'), NONE);
    assert!(ed.draft().is_none());
    assert_eq!(ed.document().elements().len(), 1);
}

#[test]
fn point_drags_are_previews_until_commit_and_undo_restores_geometry() {
    let mut ed = editing();
    let before = points(&ed);
    down(&mut ed, p(100.0, 10.0));
    motion(&mut ed, p(130.0, 30.0));
    assert_eq!(points(&ed), before);
    assert_eq!(ed.preview().len(), 1);
    up(&mut ed, p(130.0, 30.0));
    assert_eq!(points(&ed)[1], WorldPt::new(130.0, 30.0));
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(points(&ed), before);
}

#[test]
fn segment_click_adds_and_delete_removes_a_point_with_undo() {
    let mut ed = editing();
    click(&mut ed, p(55.0, 10.0));
    assert_eq!(path(&ed).points.len(), 4);
    assert_eq!(points(&ed)[1], WorldPt::new(55.0, 10.0));
    key(&mut ed, Key::Delete, NONE);
    assert_eq!(path(&ed).points.len(), 3);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(path(&ed).points.len(), 4);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(path(&ed).points.len(), 3);
}

#[test]
fn escape_cancels_a_point_drag_then_exits_point_editing() {
    let mut ed = editing();
    let before = points(&ed);
    down(&mut ed, p(100.0, 10.0));
    motion(&mut ed, p(150.0, 30.0));
    key(&mut ed, Key::Escape, NONE);
    up(&mut ed, p(150.0, 30.0));
    assert_eq!(points(&ed), before);
    assert!(ed.editable_points().is_some());
    key(&mut ed, Key::Escape, NONE);
    assert!(ed.editable_points().is_none());
    key(&mut ed, Key::Enter, CTRL);
    assert!(ed.editable_points().is_some());
}

#[test]
fn freehand_preserves_a_dot_simplifies_and_has_variable_pressure() {
    let mut ed = editor(Tool::Freedraw);
    click(&mut ed, p(20.0, 30.0));
    assert_eq!(path(&ed).points.len(), 1);
    down(&mut ed, p(0.0, 0.0));
    for i in 1..1000 {
        motion(&mut ed, p(i as f32 * 0.5, (i as f32 * 0.02).sin() * 20.0));
    }
    up(&mut ed, p(500.0, 20.0));
    let p = ed.document().elements()[1].kind.path().expect("stroke");
    assert!(p.points.len() < 500, "{}", p.points.len());
    assert!(
        p.points
            .iter()
            .all(|p| p.offset.is_finite() && p.pressure.is_finite())
    );
    assert!(p.points.iter().any(|p| (p.pressure - 0.5).abs() > 0.1));
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(ed.document().elements().len(), 1);
}

#[test]
fn escape_cancels_freehand_and_bad_moves_are_ignored() {
    let mut ed = editor(Tool::Freedraw);
    down(&mut ed, p(10.0, 10.0));
    motion(&mut ed, p(100.0, 100.0));
    motion(&mut ed, p(f32::NAN, f32::INFINITY));
    assert!(
        ed.draft()
            .expect("draft")
            .kind
            .path()
            .expect("path")
            .world_points()
            .all(WorldPt::is_finite)
    );
    key(&mut ed, Key::Escape, NONE);
    up(&mut ed, p(100.0, 100.0));
    assert!(ed.document().elements().is_empty());
}

#[test]
fn paths_move_resize_delete_and_restore_through_selection() {
    for tool in [Tool::Line, Tool::Arrow, Tool::Freedraw] {
        let mut ed = editor(tool);
        draw(&mut ed);
        let before = ed.document().elements()[0].kind.clone();
        key(&mut ed, Key::Char('a'), CTRL);
        key(&mut ed, Key::Right, NONE);
        let bounds = ed.selection_bounds().expect("selection");
        let se = ed
            .camera
            .world_to_screen(Handle::Se.position(bounds, ed.camera.zoom()));
        down(&mut ed, se);
        motion(&mut ed, p(se.x + 50.0, se.y + 30.0));
        up(&mut ed, p(se.x + 50.0, se.y + 30.0));
        assert!(
            ed.document().elements()[0]
                .kind
                .path()
                .expect("path")
                .world_points()
                .all(WorldPt::is_finite)
        );
        key(&mut ed, Key::Delete, NONE);
        assert!(ed.document().elements().is_empty());
        for _ in 0..3 {
            key(&mut ed, Key::Char('z'), CTRL);
        }
        assert_eq!(ed.document().elements()[0].kind, before);
    }
}

#[test]
fn deleting_during_point_insertion_cancels_preview_without_a_document_edit() {
    let mut ed = editing();
    let revision = ed.document.revision();
    down(&mut ed, p(55.0, 10.0));
    assert_eq!(ed.preview()[0].kind.path().expect("path").points.len(), 4);
    assert_eq!(
        ed.handle(InputEvent::Key {
            key: Key::Delete,
            mods: NONE
        }),
        Redraw::Draft
    );
    assert!(ed.preview().is_empty());
    assert_eq!(ed.document.revision(), revision);
    up(&mut ed, p(55.0, 10.0));
    assert_eq!(path(&ed).points.len(), 3);
}

#[test]
fn point_snapping_survives_camera_changes_and_handles_stay_in_screen_points() {
    let mut ed = editing();
    down(&mut ed, p(100.0, 10.0));
    motion(&mut ed, p(150.0, 40.0));
    ed.handle(InputEvent::Modifiers(SHIFT));
    ed.handle(InputEvent::Zoom {
        about: p(150.0, 40.0),
        factor: 3.0,
    });
    let (path, active) = ed.editable_points().expect("editing");
    let a = path.world_point(0).expect("anchor");
    let b = path.world_point(active.expect("active")).expect("tip");
    let steps = (b.y - a.y).atan2(b.x - a.x).to_degrees() / 15.0;
    assert!((steps - steps.round()).abs() < 1e-3);
}
