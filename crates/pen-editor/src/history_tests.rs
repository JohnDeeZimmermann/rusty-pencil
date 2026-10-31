use super::selection_tests::*;
use super::*;

#[test]
fn fifty_move_events_are_one_undo_step_and_redo_restores_selection() {
    let mut ed = one();
    let selected = ed.selection().clone();
    down(&mut ed, p(180.0, 150.0), NONE);
    for i in 1..=50 {
        movement(&mut ed, p(180.0 + i as f32, 150.0 + i as f32), NONE);
    }
    up(&mut ed, p(230.0, 200.0), NONE);
    assert_eq!(bounds(&ed), r(150.0, 150.0, 200.0, 100.0));
    key(&mut ed, Key::Escape, NONE);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
    assert_eq!(ed.selection(), &selected);
    key(
        &mut ed,
        Key::Char('z'),
        Modifiers {
            shift: true,
            ..CTRL
        },
    );
    assert_eq!(bounds(&ed), r(150.0, 150.0, 200.0, 100.0));
    assert_eq!(ed.selection(), &selected);
    key(&mut ed, Key::Char('z'), CTRL);
    key(&mut ed, Key::Char('z'), CTRL);
    assert!(
        ed.document().elements().is_empty(),
        "second undo removes creation"
    );
}

#[test]
fn resize_nudge_and_delete_restore_geometry_selection_and_z_order() {
    let mut ed = one();
    drag(&mut ed, p(305.0, 205.0), p(355.0, 225.0), NONE);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
    key(&mut ed, Key::Right, SHIFT);
    key(&mut ed, Key::Char('z'), CTRL);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
    key(&mut ed, Key::Char('r'), NONE);
    drag(&mut ed, p(400.0, 100.0), p(500.0, 200.0), NONE);
    key(&mut ed, Key::Char('a'), CTRL);
    let ids: Vec<_> = ed.selection().ids().collect();
    for delete in [Key::Delete, Key::Backspace] {
        key(&mut ed, delete, NONE);
        assert!(ed.document().elements().is_empty());
        assert!(ed.selection().is_empty());
        key(&mut ed, Key::Char('z'), CTRL);
        assert_eq!(
            ed.document()
                .elements()
                .iter()
                .map(Element::id)
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(ed.selection().ids().collect::<Vec<_>>(), ids);
        key(&mut ed, Key::Char('y'), CTRL);
        assert!(ed.document().elements().is_empty());
        key(&mut ed, Key::Char('z'), CTRL);
    }
}

#[test]
fn new_edit_discards_redo_but_cancel_and_noop_do_not() {
    let mut ed = one();
    key(&mut ed, Key::Right, NONE);
    key(&mut ed, Key::Char('z'), CTRL);
    down(&mut ed, p(180.0, 150.0), NONE);
    movement(&mut ed, p(250.0, 200.0), NONE);
    ed.handle(InputEvent::Cancel);
    click(&mut ed, p(180.0, 150.0), NONE);
    key(&mut ed, Key::Char('y'), CTRL);
    assert_eq!(bounds(&ed), r(101.0, 100.0, 200.0, 100.0));
    key(&mut ed, Key::Char('z'), CTRL);
    key(&mut ed, Key::Down, NONE);
    assert_eq!(key(&mut ed, Key::Char('y'), CTRL), Redraw::Nothing);
    assert_eq!(bounds(&ed), r(100.0, 101.0, 200.0, 100.0));
}

#[test]
fn undo_during_gesture_cancels_it_before_touching_history() {
    let mut ed = one();
    down(&mut ed, p(180.0, 150.0), NONE);
    movement(&mut ed, p(240.0, 200.0), NONE);
    key(&mut ed, Key::Char('z'), CTRL);
    up(&mut ed, p(240.0, 200.0), NONE);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
    assert!(ed.preview().is_empty());
}

#[test]
fn tool_switch_cancels_transform_and_clears_feedback() {
    let mut ed = one();
    down(&mut ed, p(180.0, 150.0), NONE);
    movement(&mut ed, p(240.0, 200.0), NONE);
    key(&mut ed, Key::Char('o'), NONE);
    up(&mut ed, p(240.0, 200.0), NONE);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
    assert!(ed.preview().is_empty());
    assert!(ed.selection().is_empty());
}
