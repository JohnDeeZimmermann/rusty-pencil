//! Selection and transforms through the public input boundary, without a renderer.
use super::*;
use pen_core::{WorldPt, WorldRect};

pub(super) const NONE: Modifiers = Modifiers::NONE;
pub(super) const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..NONE
};
pub(super) const ALT: Modifiers = Modifiers { alt: true, ..NONE };
pub(super) const CTRL: Modifiers = Modifiers { ctrl: true, ..NONE };
pub(super) fn p(x: f32, y: f32) -> ScreenPt {
    ScreenPt::new(x, y)
}
pub(super) fn r(x: f32, y: f32, w: f32, h: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x, y), WorldPt::new(x + w, y + h))
}
pub(super) fn key(ed: &mut Editor, key: Key, mods: Modifiers) -> Redraw {
    ed.handle(InputEvent::Key { key, mods })
}
pub(super) fn down(ed: &mut Editor, at: ScreenPt, mods: Modifiers) {
    ed.handle(InputEvent::PointerDown { at, mods });
}
pub(super) fn movement(ed: &mut Editor, at: ScreenPt, mods: Modifiers) {
    ed.handle(InputEvent::PointerMove { at, mods });
}
pub(super) fn up(ed: &mut Editor, at: ScreenPt, mods: Modifiers) {
    ed.handle(InputEvent::PointerUp { at, mods });
}
pub(super) fn drag(ed: &mut Editor, a: ScreenPt, b: ScreenPt, mods: Modifiers) {
    down(ed, a, mods);
    movement(ed, b, mods);
    up(ed, b, mods);
}
pub(super) fn click(ed: &mut Editor, at: ScreenPt, mods: Modifiers) {
    down(ed, at, mods);
    up(ed, at, mods);
}
pub(super) fn bounds(ed: &Editor) -> WorldRect {
    ed.document().elements()[0].kind.bounds()
}
pub(super) fn one() -> Editor {
    let mut ed = Editor::default();
    drag(&mut ed, p(100.0, 100.0), p(300.0, 200.0), NONE);
    key(&mut ed, Key::Char('v'), NONE);
    click(&mut ed, p(150.0, 100.0), NONE);
    ed
}

#[test]
fn selects_topmost_outline_and_shift_toggles() {
    let mut ed = Editor::default();
    drag(&mut ed, p(100.0, 100.0), p(300.0, 200.0), NONE);
    drag(&mut ed, p(120.0, 100.0), p(320.0, 220.0), NONE);
    key(&mut ed, Key::Char('1'), NONE);
    let ids: Vec<_> = ed.document().elements().iter().map(Element::id).collect();
    click(&mut ed, p(180.0, 100.0), NONE);
    assert_eq!(ed.selection().ids().collect::<Vec<_>>(), [ids[1]]);
    click(&mut ed, p(100.0, 150.0), SHIFT);
    assert_eq!(ed.selection().len(), 2);
    click(&mut ed, p(320.0, 160.0), SHIFT);
    assert_eq!(ed.selection().ids().collect::<Vec<_>>(), [ids[0]]);
}

#[test]
fn hollow_interior_does_not_select_an_unselected_shape() {
    let mut ed = one();
    key(&mut ed, Key::Escape, NONE);
    click(&mut ed, p(200.0, 150.0), NONE);
    assert!(ed.selection().is_empty());
}

#[test]
fn marquee_normalises_and_shift_adds_only_fully_enclosed_shapes() {
    let mut ed = one();
    key(&mut ed, Key::Char('r'), NONE);
    drag(&mut ed, p(400.0, 100.0), p(500.0, 200.0), NONE);
    key(&mut ed, Key::Char('v'), NONE);
    drag(&mut ed, p(510.0, 210.0), p(390.0, 90.0), NONE);
    assert_eq!(ed.selection().len(), 1);
    drag(&mut ed, p(80.0, 80.0), p(310.0, 210.0), SHIFT);
    assert_eq!(ed.selection().len(), 2);
    drag(&mut ed, p(80.0, 80.0), p(200.0, 180.0), NONE);
    assert!(ed.selection().is_empty(), "partial enclosure is excluded");
    key(&mut ed, Key::Char('a'), CTRL);
    assert_eq!(ed.selection().len(), 2);
}

#[test]
fn moving_previews_only_touched_elements_then_commits() {
    let mut ed = one();
    let revision = ed.document().revision();
    down(&mut ed, p(180.0, 150.0), NONE);
    movement(&mut ed, p(220.0, 180.0), NONE);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
    assert_eq!(ed.document().revision(), revision);
    assert_eq!(ed.preview()[0].kind.bounds(), r(140.0, 130.0, 200.0, 100.0));
    up(&mut ed, p(220.0, 180.0), NONE);
    assert_eq!(bounds(&ed), r(140.0, 130.0, 200.0, 100.0));
    assert!(ed.preview().is_empty());
}

#[test]
fn moving_multiple_elements_keeps_relative_positions_and_versions() {
    let mut ed = one();
    key(&mut ed, Key::Char('r'), NONE);
    drag(&mut ed, p(400.0, 100.0), p(500.0, 200.0), NONE);
    drag(&mut ed, p(600.0, 100.0), p(700.0, 200.0), NONE);
    key(&mut ed, Key::Char('v'), NONE);
    drag(&mut ed, p(80.0, 80.0), p(510.0, 220.0), NONE);
    drag(&mut ed, p(180.0, 150.0), p(200.0, 180.0), NONE);
    let elements = ed.document().elements();
    assert_eq!(elements[0].kind.bounds(), r(120.0, 130.0, 200.0, 100.0));
    assert_eq!(elements[1].kind.bounds(), r(420.0, 130.0, 100.0, 100.0));
    assert_eq!(
        elements[2].version(),
        1,
        "untouched geometry keeps its cache key"
    );
}

#[test]
fn shift_constrains_move_immediately_and_release_restores_free_motion() {
    let mut ed = one();
    down(&mut ed, p(200.0, 150.0), NONE);
    movement(&mut ed, p(240.0, 180.0), NONE);
    ed.handle(InputEvent::Modifiers(SHIFT));
    assert_eq!(ed.preview()[0].kind.bounds(), r(140.0, 100.0, 200.0, 100.0));
    ed.handle(InputEvent::Modifiers(NONE));
    assert_eq!(ed.preview()[0].kind.bounds(), r(140.0, 130.0, 200.0, 100.0));
}

#[test]
fn all_eight_handles_resize_from_the_opposite_anchor() {
    for handle in Handle::ALL {
        let mut ed = one();
        let start = handle.position(bounds(&ed), 1.0);
        let start = p(start.x, start.y);
        let (hx, hy) = handle.axes();
        let end = p(start.x + hx * 40.0, start.y + hy * 20.0);
        drag(&mut ed, start, end, NONE);
        let expected = r(
            100.0 + if hx < 0.0 { -40.0 } else { 0.0 },
            100.0 + if hy < 0.0 { -20.0 } else { 0.0 },
            200.0 + hx.abs() * 40.0,
            100.0 + hy.abs() * 20.0,
        );
        assert_eq!(bounds(&ed), expected, "{handle:?}");
    }
}

#[test]
fn shift_preserves_original_aspect_for_corner_and_side_handles() {
    for (handle, expected) in [
        (Handle::Se, r(100.0, 100.0, 300.0, 150.0)),
        (Handle::E, r(100.0, 75.0, 300.0, 150.0)),
    ] {
        let mut ed = one();
        let start = handle.position(bounds(&ed), 1.0);
        drag(
            &mut ed,
            p(start.x, start.y),
            p(start.x + 100.0, start.y + 10.0),
            SHIFT,
        );
        assert_eq!(bounds(&ed), expected);
    }
}

#[test]
fn alt_resize_is_centred_and_modifiers_update_without_pointer_motion() {
    let mut ed = one();
    down(&mut ed, p(305.0, 205.0), NONE);
    movement(&mut ed, p(355.0, 225.0), NONE);
    ed.handle(InputEvent::Modifiers(ALT));
    assert_eq!(ed.preview()[0].kind.bounds(), r(50.0, 80.0, 300.0, 140.0));
    ed.handle(InputEvent::Modifiers(Modifiers { shift: true, ..ALT }));
    assert_eq!(ed.preview()[0].kind.bounds(), r(50.0, 75.0, 300.0, 150.0));
    ed.handle(InputEvent::Modifiers(NONE));
    assert_eq!(ed.preview()[0].kind.bounds(), r(100.0, 100.0, 250.0, 120.0));
}

#[test]
fn crossing_resize_anchor_flips_without_degenerate_shapes() {
    let mut ed = one();
    drag(&mut ed, p(305.0, 205.0), p(55.0, 55.0), NONE);
    assert_eq!(bounds(&ed), r(50.0, 50.0, 50.0, 50.0));
    let start = Handle::Se.position(bounds(&ed), 1.0);
    drag(
        &mut ed,
        p(start.x, start.y),
        p(start.x - 50.0, start.y - 50.0),
        NONE,
    );
    assert!(!bounds(&ed).is_degenerate());
}

#[test]
fn cancel_restores_selection_and_geometry_for_move_resize_and_box() {
    for (a, b) in [
        (p(200.0, 150.0), p(240.0, 200.0)),
        (p(305.0, 205.0), p(350.0, 250.0)),
        (p(50.0, 50.0), p(400.0, 300.0)),
    ] {
        let mut ed = one();
        let before = ed.selection().clone();
        down(&mut ed, a, NONE);
        movement(&mut ed, b, NONE);
        key(&mut ed, Key::Escape, NONE);
        up(&mut ed, b, NONE);
        assert_eq!(ed.selection(), &before);
        assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
        assert!(ed.preview().is_empty());
        assert!(ed.marquee().is_none());
    }
}

#[test]
fn nudge_is_in_screen_pixels_at_any_zoom() {
    let mut ed = one();
    ed.handle(InputEvent::Zoom {
        about: p(0.0, 0.0),
        factor: 2.0,
    });
    key(&mut ed, Key::Right, NONE);
    key(&mut ed, Key::Down, SHIFT);
    assert_eq!(bounds(&ed), r(100.5, 105.0, 200.0, 100.0));
}

#[test]
fn move_under_zoom_and_pan_keeps_pointer_anchor() {
    let mut ed = one();
    ed.handle(InputEvent::Zoom {
        about: p(0.0, 0.0),
        factor: 2.0,
    });
    down(&mut ed, p(400.0, 300.0), NONE);
    movement(&mut ed, p(440.0, 320.0), NONE);
    ed.handle(InputEvent::Pan {
        by: ScreenVec::new(-20.0, 0.0),
    });
    up(&mut ed, p(440.0, 320.0), NONE);
    assert_eq!(bounds(&ed), r(130.0, 110.0, 200.0, 100.0));
}

#[test]
fn tiny_click_and_nonfinite_move_leave_geometry_unchanged() {
    let mut ed = one();
    drag(&mut ed, p(180.0, 150.0), p(181.0, 150.0), NONE);
    down(&mut ed, p(180.0, 150.0), NONE);
    movement(&mut ed, p(f32::NAN, f32::INFINITY), NONE);
    up(&mut ed, p(180.0, 150.0), NONE);
    assert_eq!(bounds(&ed), r(100.0, 100.0, 200.0, 100.0));
}

#[test]
fn diamond_and_ellipse_resize_keep_their_kind_and_seed() {
    for (tool, kind) in [
        (Tool::Diamond, Kind::Diamond as fn(WorldRect) -> Kind),
        (Tool::Ellipse, Kind::Ellipse),
    ] {
        let mut ed = Editor::default();
        ed.handle(InputEvent::SelectTool(tool));
        drag(&mut ed, p(100.0, 100.0), p(300.0, 200.0), NONE);
        key(&mut ed, Key::Char('a'), CTRL);
        let seed = ed.document().elements()[0].seed();
        drag(&mut ed, p(305.0, 205.0), p(355.0, 225.0), NONE);
        assert_eq!(
            ed.document().elements()[0].kind,
            kind(r(100.0, 100.0, 250.0, 120.0))
        );
        assert_eq!(ed.document().elements()[0].seed(), seed);
    }
}

#[test]
fn focus_cancel_preserves_an_idle_selection() {
    let mut ed = one();
    let before = ed.selection().clone();
    assert_eq!(ed.handle(InputEvent::Cancel), Redraw::Nothing);
    assert_eq!(ed.selection(), &before);
}
