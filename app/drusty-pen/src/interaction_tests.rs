//! The actual Rune handlers, including native command keys and live overlay presentation.
use super::*;

fn command(rt: &mut Runtime<State>, c: char, shift: bool) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: Mods {
            ctrl: true,
            cmd: true,
            shift,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}
fn special(rt: &mut Runtime<State>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}
fn selected_app() -> Runtime<State> {
    let mut rt = app();
    drag(&mut rt, (300.0, 300.0), (500.0, 400.0));
    key(&mut rt, "v");
    command(&mut rt, 'a', false);
    rt
}
fn overlay_rects(rt: &Runtime<State>) -> Vec<[f32; 4]> {
    rt.elements()
        .into_iter()
        .filter(|e| {
            e.key
                .as_deref()
                .is_some_and(|k| k.starts_with("selection-") && k != "selection-overlay")
        })
        .map(|e| e.rect)
        .collect()
}

#[test]
fn native_commands_select_delete_undo_redo_and_nudge() {
    let mut rt = selected_app();
    assert_eq!(rt.state.editor.tool(), Tool::Select);
    assert_eq!(rt.state.editor.selection().len(), 1);
    special(&mut rt, Key::Right);
    assert_eq!(kinds(&rt), [Kind::Rect(world(301.0, 300.0, 501.0, 400.0))]);
    special(&mut rt, Key::Delete);
    assert!(kinds(&rt).is_empty());
    command(&mut rt, 'z', false);
    assert_eq!(rt.state.editor.selection().len(), 1);
    command(&mut rt, 'z', true);
    assert!(kinds(&rt).is_empty());
    command(&mut rt, 'z', false);
    command(&mut rt, 'y', false);
    assert!(kinds(&rt).is_empty());
}

#[test]
fn selection_overlay_pans_and_zooms_live_without_view_rebuild() {
    let mut rt = selected_app();
    let before = overlay_rects(&rt);
    assert_eq!(before.len(), 9);
    let commits = rt.stats.commits;
    rt.wheel_event(WheelEvent::new(30.0, -20.0).at(100.0, 500.0));
    rt.frame(FRAME);
    rt.magnify_event(MagnifyEvent::new(1.0).at(512.0, 360.0));
    rt.frame(FRAME);
    assert_eq!(
        rt.stats.commits, commits,
        "camera updates must remain live-only with handles"
    );
    let after = overlay_rects(&rt);
    assert_eq!(after.len(), 9);
    assert!(near(&after[0][2..], &[410.0, 210.0]));
    for handle in &after[1..] {
        assert!(near(&handle[2..], &[8.0, 8.0]));
    }
}

#[test]
fn dragging_selected_shape_and_handle_commits_then_undoes() {
    let mut rt = selected_app();
    drag(&mut rt, (400.0, 350.0), (450.0, 380.0));
    assert_eq!(kinds(&rt), [Kind::Rect(world(350.0, 330.0, 550.0, 430.0))]);
    drag(&mut rt, (555.0, 435.0), (615.0, 465.0));
    assert_eq!(kinds(&rt), [Kind::Rect(world(350.0, 330.0, 610.0, 460.0))]);
    command(&mut rt, 'z', false);
    assert_eq!(kinds(&rt), [Kind::Rect(world(350.0, 330.0, 550.0, 430.0))]);
    command(&mut rt, 'z', false);
    assert_eq!(kinds(&rt), [Kind::Rect(world(300.0, 300.0, 500.0, 400.0))]);
    assert_eq!(overlay_rects(&rt).len(), 9);
}

#[test]
fn escape_and_toolbar_switch_clear_the_live_transform() {
    let mut rt = selected_app();
    rt.pointer_move(400.0, 350.0);
    rt.pointer_down(400.0, 350.0);
    rt.pointer_move(460.0, 380.0);
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.preview().len(), 1);
    special(&mut rt, Key::Escape);
    rt.pointer_up(460.0, 380.0);
    rt.frame(FRAME);
    assert!(rt.state.editor.preview().is_empty());
    assert_eq!(kinds(&rt), [Kind::Rect(world(300.0, 300.0, 500.0, 400.0))]);
    key(&mut rt, "r");
    assert_eq!(rt.state.editor.tool(), Tool::Rectangle);
    assert!(overlay_rects(&rt).is_empty(), "{:?}", overlay_rects(&rt));
}

#[test]
fn marquee_and_shift_click_work_through_rune_pointer_dispatch() {
    let mut rt = app();
    drag(&mut rt, (200.0, 300.0), (350.0, 400.0));
    drag(&mut rt, (500.0, 300.0), (650.0, 400.0));
    key(&mut rt, "v");
    drag(&mut rt, (180.0, 280.0), (360.0, 420.0));
    assert_eq!(rt.state.editor.selection().len(), 1);
    shift(&mut rt, true);
    drag(&mut rt, (550.0, 300.0), (550.0, 300.0));
    shift(&mut rt, false);
    assert_eq!(rt.state.editor.selection().len(), 2);
}

#[test]
fn resize_modifiers_change_feedback_on_the_same_frame() {
    let mut rt = selected_app();
    rt.pointer_move(505.0, 405.0);
    rt.pointer_down(505.0, 405.0);
    rt.pointer_move(565.0, 425.0);
    rt.frame(FRAME);
    shift(&mut rt, true);
    rt.physical_input(PhysicalInput::Key {
        key: PhysicalKeyCode::AltLeft,
        mods: Mods {
            shift: true,
            alt: true,
            ..Mods::default()
        },
        pressed: true,
        repeat: false,
    });
    rt.frame(FRAME);
    assert_eq!(
        rt.state.editor.preview()[0].kind,
        Kind::Rect(world(240.0, 270.0, 560.0, 430.0))
    );
    rt.pointer_up(565.0, 425.0);
    rt.frame(FRAME);
    assert_eq!(kinds(&rt), [Kind::Rect(world(240.0, 270.0, 560.0, 430.0))]);
}
