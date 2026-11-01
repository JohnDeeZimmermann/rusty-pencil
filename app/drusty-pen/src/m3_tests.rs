//! Integration tests exercise Rune dispatch, including hover tails and real key releases.
use super::*;
use rune::PointerButton;

fn command(rt: &mut Runtime<State>, c: char) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: Mods {
            cmd: true,
            ctrl: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
}
fn click(rt: &mut Runtime<State>, x: f32, y: f32) {
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(FRAME);
}
fn space(rt: &mut Runtime<State>, pressed: bool) {
    let ev = KeyEvent {
        key: Key::Text(" ".into()),
        mods: Mods::default(),
    };
    if pressed {
        rt.key(ev);
    } else {
        rt.key_up(ev);
    }
    rt.frame(FRAME);
}

#[test]
fn line_hover_tail_finishes_on_enter_and_arrow_uses_shortcut_five() {
    let mut rt = app();
    key(&mut rt, "l");
    click(&mut rt, 250.0, 250.0);
    click(&mut rt, 500.0, 250.0);
    rt.pointer_move(600.0, 400.0);
    rt.frame(FRAME);
    let draft = rt.state.editor.draft().expect("line draft");
    assert_eq!(draft.kind.path().expect("path").points.len(), 3);
    rt.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert_eq!(kinds(&rt)[0].path().expect("line").points.len(), 3);
    key(&mut rt, "5");
    drag(&mut rt, (300.0, 450.0), (600.0, 450.0));
    assert!(matches!(&kinds(&rt)[1], Kind::Arrow(_)));
    assert!(!rt.state.scene.is_empty());
}

#[test]
fn middle_and_hand_drags_are_live_only_and_space_release_restores_drawing() {
    let mut rt = app();
    // Focusing the canvas is a UI change; subsequent navigation is entirely live-only.
    click(&mut rt, 50.0, 200.0);
    let commits = rt.stats.commits;
    let before = *rt.state.editor.camera();
    rt.button_down(300.0, 300.0, PointerButton::Middle);
    rt.frame(FRAME);
    rt.pointer_move(400.0, 350.0);
    rt.frame(FRAME);
    rt.button_up(400.0, 350.0, PointerButton::Middle);
    rt.frame(FRAME);
    assert_eq!(rt.stats.commits, commits, "middle pan stays live-only");
    assert_ne!(*rt.state.editor.camera(), before);
    key(&mut rt, "h");
    let commits = rt.stats.commits;
    drag(&mut rt, (300.0, 300.0), (350.0, 320.0));
    assert_eq!(rt.stats.commits, commits, "hand pan stays live-only");
    key(&mut rt, "r");
    space(&mut rt, true);
    drag(&mut rt, (300.0, 300.0), (350.0, 320.0));
    assert!(kinds(&rt).is_empty());
    space(&mut rt, false);
    assert!(!rt.state.editor.space_held());
    drag(&mut rt, (300.0, 300.0), (400.0, 400.0));
    assert_eq!(kinds(&rt).len(), 1);
}

#[test]
fn ctrl_enter_edits_line_points_and_double_click_labels_through_rune() {
    let mut rt = app();
    key(&mut rt, "l");
    drag(&mut rt, (300.0, 300.0), (600.0, 400.0));
    key(&mut rt, "v");
    click(&mut rt, 450.0, 350.0);
    click(&mut rt, 450.0, 350.0);
    assert!(
        rt.state.editor.text_session().is_some(),
        "double-click starts a label"
    );
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert_eq!(kinds(&rt).len(), 1, "an empty label adds nothing");
    rt.key(KeyEvent {
        key: Key::Enter,
        mods: Mods {
            cmd: true,
            ctrl: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
    assert!(
        rt.state.editor.editable_points().is_some(),
        "Ctrl+Enter opens points"
    );
    drag(&mut rt, (600.0, 400.0), (650.0, 450.0));
    let path = kinds(&rt)[0].path().expect("line").clone();
    assert_eq!(
        path.world_point(1),
        Some(WorldPt::new(650.0 - WINDOW.0 / 2.0, 450.0 - WINDOW.1 / 2.0))
    );
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    rt.key(KeyEvent {
        key: Key::Enter,
        mods: Mods {
            cmd: true,
            ctrl: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
    assert!(rt.state.editor.editable_points().is_some());
    // Midpoint handles use the same canvas event route to insert a point.
    click(&mut rt, 475.0, 375.0);
    assert_eq!(kinds(&rt)[0].path().expect("line").points.len(), 3);
    rt.key(KeyEvent {
        key: Key::Delete,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert_eq!(kinds(&rt)[0].path().expect("line").points.len(), 2);
}

#[test]
fn zoom_commands_and_return_to_content_do_not_rebuild_the_view() {
    let mut rt = app();
    drag(&mut rt, (300.0, 300.0), (600.0, 400.0));
    let commits = rt.stats.commits;
    command(&mut rt, '+');
    assert_eq!(zoom_text(&rt), "120%");
    command(&mut rt, '0');
    assert_eq!(zoom_text(&rt), "100%");
    rt.wheel_event(WheelEvent::new(5000.0, 0.0).at(100.0, 500.0));
    rt.frame(FRAME);
    assert!(rt.state.editor.content_offscreen());
    assert_eq!(
        rt.stats.commits, commits,
        "return button visibility stays live-only"
    );
    let (x, y) = rt.find("content-return").expect("return button");
    click(&mut rt, x, y);
    assert!(!rt.state.editor.content_offscreen());
    assert_eq!(rt.stats.commits, commits, "fit content stays live-only");
}

#[test]
fn freehand_renders_live_commits_once_and_undoes() {
    let mut rt = app();
    key(&mut rt, "7");
    rt.pointer_down(250.0, 400.0);
    for i in 1..100 {
        rt.pointer_move(
            250.0 + i as f32 * 3.0,
            400.0 + (i as f32 * 0.08).sin() * 60.0,
        );
        rt.frame(FRAME);
    }
    assert!(!rt.state.draft.is_empty());
    assert!(kinds(&rt).is_empty());
    rt.pointer_up(550.0, 460.0);
    rt.frame(FRAME);
    assert!(matches!(&kinds(&rt)[0], Kind::Freedraw(_)));
    command(&mut rt, 'z');
    assert!(kinds(&rt).is_empty());
}

#[test]
fn paint_survives_live_zoom_when_the_world_origin_is_offscreen() {
    for tool in ["r", "a", "l", "p"] {
        let mut rt = app();
        key(&mut rt, tool);
        drag(&mut rt, (300.0, 300.0), (600.0, 400.0));
        let commits = rt.stats.commits;
        rt.magnify_event(
            MagnifyEvent::new(29.0).at(450.0, if tool == "r" { 300.0 } else { 350.0 }),
        );
        rt.frame(FRAME);
        rt.frame(FRAME);
        assert_eq!(rt.stats.commits, commits, "{tool}: zoom stays live-only");
        let origin = rt.state.editor.camera().world_to_screen(WorldPt::ORIGIN);
        assert!(
            origin.x > WINDOW.0 || origin.y > WINDOW.1,
            "origin is outside the window"
        );
        // Check actual paint emission, not only layout transforms. Chrome is outside this
        // central region; without bounded canvas nodes Rune culls every scene primitive.
        assert!(
            rt.instances.iter().any(|i| {
                i.fill[0] < 0.15
                    && i.fill[1] < 0.15
                    && i.fill[2] < 0.15
                    && i.fill[3] > 0.99
                    && i.rect[0] < 1400.0
                    && i.rect[0] + i.rect[2] > 600.0
                    && i.rect[1] < 1100.0
                    && i.rect[1] + i.rect[3] > 300.0
            }),
            "{tool}: document geometry must still be emitted"
        );
    }
}
