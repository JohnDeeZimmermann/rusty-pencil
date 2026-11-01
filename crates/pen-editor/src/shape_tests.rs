//! The shape tools, driven only through `InputEvent`s.

use super::*;
use pen_core::{WorldPt, WorldRect};

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::NONE
};
const ALT: Modifiers = Modifiers {
    alt: true,
    ..Modifiers::NONE
};

/// A camera with screen == world, so expectations read directly.
fn editor() -> Editor {
    Editor::new(Camera::default())
}

fn pt(x: f32, y: f32) -> ScreenPt {
    ScreenPt::new(x, y)
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x0, y0), WorldPt::new(x1, y1))
}

fn drag(ed: &mut Editor, from: ScreenPt, to: ScreenPt, mods: Modifiers) -> Redraw {
    ed.handle(InputEvent::PointerDown { at: from, mods });
    let mid = pt((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
    ed.handle(InputEvent::PointerMove { at: mid, mods });
    ed.handle(InputEvent::PointerMove { at: to, mods });
    ed.handle(InputEvent::PointerUp { at: to, mods })
}

fn only_kind(ed: &Editor) -> Kind {
    match ed.document().elements() {
        [e] => e.kind.clone(),
        other => panic!("expected one element, got {other:?}"),
    }
}

fn key(c: char) -> InputEvent {
    InputEvent::Key {
        key: Key::Char(c),
        mods: NONE,
    }
}

#[test]
fn each_tool_draws_its_kind() {
    for (tool, expect) in [
        (Tool::Rectangle, Kind::Rect as fn(WorldRect) -> Kind),
        (Tool::Diamond, Kind::Diamond),
        (Tool::Ellipse, Kind::Ellipse),
    ] {
        let mut ed = editor();
        ed.handle(InputEvent::SelectTool(tool));
        assert_eq!(
            drag(&mut ed, pt(10.0, 20.0), pt(110.0, 70.0), NONE),
            Redraw::Scene
        );
        assert_eq!(only_kind(&ed), expect(rect(10.0, 20.0, 110.0, 70.0)));
        assert_eq!(ed.draft(), None, "{tool:?}: commit clears the draft");
    }
}

#[test]
fn keys_switch_tools() {
    let mut ed = editor();
    for (c, tool) in [
        ('d', Tool::Diamond),
        ('o', Tool::Ellipse),
        ('r', Tool::Rectangle),
        ('3', Tool::Diamond),
        ('4', Tool::Ellipse),
        ('2', Tool::Rectangle),
        ('D', Tool::Diamond),
    ] {
        ed.handle(key(c));
        assert_eq!(ed.tool(), tool, "key {c:?}");
    }
    assert_eq!(ed.handle(key('d')), Redraw::Nothing, "same tool again");
    assert_eq!(ed.handle(key('x')), Redraw::Nothing);
    // Ctrl+D is a command (duplicate, M6), not the diamond tool.
    ed.handle(key('r'));
    let ctrl = Modifiers { ctrl: true, ..NONE };
    ed.handle(InputEvent::Key {
        key: Key::Char('d'),
        mods: ctrl,
    });
    assert_eq!(ed.tool(), Tool::Rectangle);
}

#[test]
fn dragging_shows_a_draft_and_release_commits_it() {
    let mut ed = editor();
    ed.handle(InputEvent::PointerDown {
        at: pt(0.0, 0.0),
        mods: NONE,
    });
    assert_eq!(
        ed.handle(InputEvent::PointerMove {
            at: pt(40.0, 30.0),
            mods: NONE
        }),
        Redraw::Draft
    );
    let draft = ed.draft().expect("drawing");
    assert_eq!(draft.kind, Kind::Rect(rect(0.0, 0.0, 40.0, 30.0)));
    assert!(ed.document().elements().is_empty(), "not committed yet");
    ed.handle(InputEvent::PointerUp {
        at: pt(40.0, 30.0),
        mods: NONE,
    });
    assert_eq!(only_kind(&ed), Kind::Rect(rect(0.0, 0.0, 40.0, 30.0)));
}

#[test]
fn dragging_up_and_left_normalises() {
    let mut ed = editor();
    drag(&mut ed, pt(100.0, 100.0), pt(20.0, 60.0), NONE);
    assert_eq!(only_kind(&ed), Kind::Rect(rect(20.0, 60.0, 100.0, 100.0)));
}

#[test]
fn shift_keeps_a_square_on_the_longer_side() {
    let mut ed = editor();
    drag(&mut ed, pt(0.0, 0.0), pt(80.0, 30.0), SHIFT);
    assert_eq!(only_kind(&ed), Kind::Rect(rect(0.0, 0.0, 80.0, 80.0)));

    // Up and to the left: the square grows towards the pointer.
    let mut ed = editor();
    ed.handle(InputEvent::SelectTool(Tool::Ellipse));
    drag(&mut ed, pt(0.0, 0.0), pt(-20.0, -50.0), SHIFT);
    assert_eq!(only_kind(&ed), Kind::Ellipse(rect(-50.0, -50.0, 0.0, 0.0)));
}

#[test]
fn alt_draws_from_the_centre() {
    let mut ed = editor();
    drag(&mut ed, pt(100.0, 100.0), pt(130.0, 110.0), ALT);
    assert_eq!(only_kind(&ed), Kind::Rect(rect(70.0, 90.0, 130.0, 110.0)));
}

#[test]
fn shift_and_alt_together_give_a_centred_square() {
    let mut ed = editor();
    ed.handle(InputEvent::SelectTool(Tool::Diamond));
    let both = Modifiers {
        shift: true,
        alt: true,
        ..NONE
    };
    drag(&mut ed, pt(0.0, 0.0), pt(10.0, -25.0), both);
    assert_eq!(
        only_kind(&ed),
        Kind::Diamond(rect(-25.0, -25.0, 25.0, 25.0))
    );
}

#[test]
fn pressing_shift_mid_drag_updates_the_draft_without_moving() {
    let mut ed = editor();
    ed.handle(InputEvent::PointerDown {
        at: pt(0.0, 0.0),
        mods: NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: pt(60.0, 20.0),
        mods: NONE,
    });
    assert_eq!(ed.handle(InputEvent::Modifiers(SHIFT)), Redraw::Draft);
    assert_eq!(
        ed.draft().map(|d| d.kind),
        Some(Kind::Rect(rect(0.0, 0.0, 60.0, 60.0)))
    );
    assert_eq!(ed.handle(InputEvent::Modifiers(SHIFT)), Redraw::Nothing);
    // Modifier changes with no drag do nothing.
    let mut idle = editor();
    assert_eq!(idle.handle(InputEvent::Modifiers(SHIFT)), Redraw::Nothing);
}

#[test]
fn a_click_creates_nothing() {
    let mut ed = editor();
    assert_eq!(
        drag(&mut ed, pt(50.0, 50.0), pt(50.0, 50.0), NONE),
        Redraw::Draft
    );
    // A wobble within the click slop is still a click.
    drag(&mut ed, pt(50.0, 50.0), pt(51.5, 52.0), NONE);
    // A flat line has no area either.
    drag(&mut ed, pt(0.0, 10.0), pt(200.0, 10.0), NONE);
    assert!(ed.document().elements().is_empty());
    assert_eq!(ed.draft(), None);
}

#[test]
fn escape_cancels_the_drag() {
    let mut ed = editor();
    ed.handle(InputEvent::PointerDown {
        at: pt(0.0, 0.0),
        mods: NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: pt(50.0, 50.0),
        mods: NONE,
    });
    assert_eq!(
        ed.handle(InputEvent::Key {
            key: Key::Escape,
            mods: NONE
        }),
        Redraw::Draft
    );
    assert_eq!(ed.draft(), None);
    // The release that follows a cancel commits nothing.
    assert_eq!(
        ed.handle(InputEvent::PointerUp {
            at: pt(50.0, 50.0),
            mods: NONE
        }),
        Redraw::Nothing
    );
    assert!(ed.document().elements().is_empty());
    assert_eq!(ed.handle(InputEvent::Cancel), Redraw::Nothing);
}

#[test]
fn drawing_under_zoom_uses_world_points() {
    let mut ed = Editor::new(Camera::default());
    ed.handle(InputEvent::Zoom {
        about: pt(0.0, 0.0),
        factor: 2.0,
    });
    drag(&mut ed, pt(20.0, 20.0), pt(220.0, 120.0), NONE);
    assert_eq!(only_kind(&ed), Kind::Rect(rect(10.0, 10.0, 110.0, 60.0)));
    // The click slop is in screen points: 2 px at 200% is a click.
    drag(&mut ed, pt(0.0, 0.0), pt(2.0, 2.0), NONE);
    assert_eq!(ed.document().elements().len(), 1);
}

#[test]
fn panning_mid_drag_keeps_the_corner_under_the_pointer() {
    let mut ed = editor();
    ed.handle(InputEvent::PointerDown {
        at: pt(0.0, 0.0),
        mods: NONE,
    });
    ed.handle(InputEvent::PointerMove {
        at: pt(50.0, 50.0),
        mods: NONE,
    });
    let redraw = ed.handle(InputEvent::Pan {
        by: ScreenVec::new(-30.0, 0.0),
    });
    assert_eq!(redraw, Redraw::Draft);
    // Content moved left 30, so the pointer now sits over world x = 80.
    assert_eq!(
        ed.draft().map(|d| d.kind),
        Some(Kind::Rect(rect(0.0, 0.0, 80.0, 50.0)))
    );
}

#[test]
fn new_elements_stack_on_top() {
    let mut ed = editor();
    drag(&mut ed, pt(0.0, 0.0), pt(10.0, 10.0), NONE);
    drag(&mut ed, pt(5.0, 5.0), pt(20.0, 20.0), NONE);
    let ids: Vec<_> = ed.document().elements().iter().map(Element::id).collect();
    assert!(ids[0] < ids[1]);
}
