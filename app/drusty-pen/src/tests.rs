//! Headless app tests: the real view and handlers, driven through `rune::Runtime`.

use super::*;
use pen_core::{Kind, WorldRect};
use pen_editor::Tool;
use rune::Color;
use rune::{Env, PhysicalInput, PhysicalKeyCode, Runtime};

fn app() -> Runtime<State> {
    let camera = Camera::centred_on(WorldPt::ORIGIN, ScreenVec::new(WINDOW.0, WINDOW.1), 1.0);
    let mut rt = Runtime::new(State::new(camera), view, WINDOW, Env::new(false, 2.0));
    rt.on_key = spec().on_key;
    rt.set_modifier_keys(true);
    rt.check_live_only(false);
    rt.frame(0.0);
    rt
}

const FRAME: f64 = 1.0 / 60.0;

fn drag(rt: &mut Runtime<State>, from: (f32, f32), to: (f32, f32)) {
    rt.pointer_move(from.0, from.1);
    rt.pointer_down(from.0, from.1);
    rt.frame(FRAME);
    rt.pointer_move((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
    rt.frame(FRAME);
    rt.pointer_move(to.0, to.1);
    rt.frame(FRAME);
    rt.pointer_up(to.0, to.1);
    rt.frame(FRAME);
}

/// The world rect under a screen rect, with the opening camera (world origin mid-window).
fn world(x0: f32, y0: f32, x1: f32, y1: f32) -> WorldRect {
    let (cx, cy) = (WINDOW.0 / 2.0, WINDOW.1 / 2.0);
    WorldRect::from_corners(
        WorldPt::new(x0 - cx, y0 - cy),
        WorldPt::new(x1 - cx, y1 - cy),
    )
}

fn kinds(rt: &Runtime<State>) -> Vec<Kind> {
    rt.state
        .editor
        .document()
        .elements()
        .iter()
        .map(|e| e.kind)
        .collect()
}

fn key(rt: &mut Runtime<State>, c: &str) {
    rt.key(KeyEvent {
        key: Key::Text(c.into()),
        mods: Mods::default(),
    });
    rt.frame(FRAME);
}

fn shift(rt: &mut Runtime<State>, down: bool) {
    rt.physical_input(PhysicalInput::Key {
        key: PhysicalKeyCode::ShiftLeft,
        mods: Mods {
            shift: down,
            ..Mods::default()
        },
        pressed: down,
        repeat: false,
    });
    rt.frame(FRAME);
}

fn near(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-3)
}

fn world_rect(rt: &Runtime<State>) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("world"))
        .expect("world node")
        .rect
}

fn zoom_text(rt: &Runtime<State>) -> String {
    rt.elements()
        .into_iter()
        .filter_map(|e| e.text)
        .find(|t| t.ends_with('%'))
        .unwrap_or_default()
}

#[test]
fn opens_with_the_world_origin_centred() {
    let rt = app();
    let [x, y, w, h] = world_rect(&rt);
    // The world node sits at the camera offset; its origin is the world origin.
    assert!(near(
        &[x, y, w, h],
        &[WINDOW.0 / 2.0, WINDOW.1 / 2.0, 0.0, 0.0]
    ));
    assert_eq!(zoom_text(&rt), "100%");
}

#[test]
fn wheel_pans_without_rebuilding_the_view() {
    let mut rt = app();
    let commits = rt.stats.commits;
    rt.wheel_event(WheelEvent::new(40.0, -25.0).at(300.0, 300.0));
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.commits, commits, "a pan is live-only");
    let [x, y, ..] = world_rect(&rt);
    assert!(
        near(&[x, y], &[WINDOW.0 / 2.0 + 40.0, WINDOW.1 / 2.0 - 25.0]),
        "{x}, {y}"
    );
}

#[test]
fn pinch_zooms_about_the_cursor_without_rebuilding_the_view() {
    let mut rt = app();
    let commits = rt.stats.commits;
    let cursor = ScreenPt::new(700.0, 200.0);
    let under = rt.state.editor.camera().screen_to_world(cursor);
    rt.magnify_event(MagnifyEvent::new(1.0).at(cursor.x, cursor.y));
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.commits, commits, "a zoom is live-only");
    let cam = *rt.state.editor.camera();
    assert!((cam.zoom() - 2.0).abs() < 1e-6);
    let after = cam.screen_to_world(cursor);
    assert!((after.x - under.x).abs() < 1e-3 && (after.y - under.y).abs() < 1e-3);
    // Rune presents the same transform the editor computed.
    let world = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("world"))
        .expect("world node");
    let origin = cam.world_to_screen(WorldPt::ORIGIN);
    assert!((world.rect[0] - origin.x).abs() < 1e-3 && (world.rect[1] - origin.y).abs() < 1e-3);
    assert!((world.scale - 2.0).abs() < 1e-6);
    assert_eq!(zoom_text(&rt), "200%");
}

#[test]
fn camera_arg_parses_and_rejects_garbage() {
    let args = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
    let cam = present::camera_arg(&args("--camera 10,-20,2")).expect("valid");
    assert!((cam.zoom() - 2.0).abs() < 1e-6);
    assert!(present::camera_arg(&args("--camera 1,2")).is_none());
    assert!(present::camera_arg(&args("--camera a,b,c")).is_none());
    assert!(present::camera_arg(&args("--other")).is_none());
}

#[test]
fn srgb_hex_decodes_to_linear() {
    let c = palette::srgb(0x1e_1e_1e);
    assert!((c.r - 0.0129).abs() < 1e-3, "{c:?}");
    assert_eq!(palette::srgb(0xff_ff_ff), Color::WHITE);
    assert_eq!(
        rune::color::hex("#1e1e1e").map(|h| (h.r * 1e4).round()),
        Some((c.r * 1e4).round())
    );
}

#[test]
fn dragging_on_the_canvas_draws_a_rectangle() {
    let mut rt = app();
    drag(&mut rt, (300.0, 300.0), (420.0, 380.0));
    assert_eq!(kinds(&rt), [Kind::Rect(world(300.0, 300.0, 420.0, 380.0))]);
    assert!(!rt.state.scene.is_empty(), "the scene shows the element");
    assert!(rt.state.draft.is_empty(), "the draft is gone after release");
}

#[test]
fn the_draft_shows_while_dragging() {
    let mut rt = app();
    rt.pointer_move(300.0, 300.0);
    rt.pointer_down(300.0, 300.0);
    rt.pointer_move(400.0, 400.0);
    rt.frame(FRAME);
    assert!(!rt.state.draft.is_empty());
    assert!(kinds(&rt).is_empty(), "nothing committed mid-drag");
    // Escape abandons the drag; the release commits nothing.
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    rt.pointer_up(400.0, 400.0);
    rt.frame(FRAME);
    assert!(rt.state.draft.is_empty());
    assert!(kinds(&rt).is_empty());
}

#[test]
fn keys_and_buttons_switch_tools() {
    let mut rt = app();
    key(&mut rt, "d");
    assert_eq!(rt.state.editor.tool(), Tool::Diamond);
    key(&mut rt, "4");
    assert_eq!(rt.state.editor.tool(), Tool::Ellipse);

    let (x, y) = rt.find("tool-r").expect("rectangle button");
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.tool(), Tool::Rectangle);
    assert!(kinds(&rt).is_empty(), "a toolbar click draws nothing");

    key(&mut rt, "o");
    drag(&mut rt, (200.0, 300.0), (300.0, 360.0));
    assert_eq!(
        kinds(&rt),
        [Kind::Ellipse(world(200.0, 300.0, 300.0, 360.0))]
    );
}

#[test]
fn a_drag_starting_on_the_toolbar_draws_nothing() {
    let mut rt = app();
    let [x, y, w, _] = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("tools"))
        .expect("tool island")
        .rect;
    // Between the island's edge and its first button.
    drag(&mut rt, (x + 2.0, y + 2.0), (x + w + 200.0, y + 200.0));
    assert!(kinds(&rt).is_empty());
}

#[test]
fn a_drag_starting_beside_the_toolbar_still_draws() {
    let mut rt = app();
    // The toolbar row spans the window's width; only the island blocks the canvas.
    drag(&mut rt, (60.0, 30.0), (160.0, 130.0));
    assert_eq!(kinds(&rt).len(), 1);
}

#[test]
fn a_click_draws_nothing() {
    let mut rt = app();
    drag(&mut rt, (300.0, 300.0), (301.0, 300.0));
    assert!(kinds(&rt).is_empty());
}

#[test]
fn shift_held_from_the_start_draws_a_square() {
    let mut rt = app();
    shift(&mut rt, true);
    drag(&mut rt, (300.0, 300.0), (400.0, 340.0));
    shift(&mut rt, false);
    assert_eq!(kinds(&rt), [Kind::Rect(world(300.0, 300.0, 400.0, 400.0))]);
}

#[test]
fn pressing_shift_mid_drag_squares_the_draft_at_once() {
    let mut rt = app();
    rt.pointer_move(300.0, 300.0);
    rt.pointer_down(300.0, 300.0);
    rt.pointer_move(400.0, 340.0);
    rt.frame(FRAME);
    shift(&mut rt, true);
    let draft = rt.state.editor.draft().expect("drawing");
    assert_eq!(draft.kind, Kind::Rect(world(300.0, 300.0, 400.0, 400.0)));
    rt.pointer_up(400.0, 340.0);
    rt.frame(FRAME);
    shift(&mut rt, false);
    assert_eq!(kinds(&rt), [Kind::Rect(world(300.0, 300.0, 400.0, 400.0))]);
}

#[test]
fn ctrl_wheel_zooms_about_the_cursor_without_rebuilding_the_view() {
    let mut rt = app();
    let commits = rt.stats.commits;
    let ctrl = Mods {
        ctrl: true,
        cmd: !cfg!(target_os = "macos"),
        ..Mods::default()
    };
    let cursor = ScreenPt::new(200.0, 500.0);
    let under = rt.state.editor.camera().screen_to_world(cursor);
    rt.wheel_event(
        WheelEvent::new(0.0, 100.0)
            .at(cursor.x, cursor.y)
            .mods(ctrl),
    );
    rt.frame(FRAME);
    rt.frame(FRAME);
    assert_eq!(rt.stats.commits, commits, "a zoom is live-only");
    let cam = *rt.state.editor.camera();
    assert!(cam.zoom() > 1.5, "{}", cam.zoom());
    let after = cam.screen_to_world(cursor);
    assert!((after.x - under.x).abs() < 1e-3 && (after.y - under.y).abs() < 1e-3);
}

#[test]
fn shapes_drawn_while_zoomed_are_in_world_points() {
    let mut rt = app();
    rt.magnify_event(MagnifyEvent::new(1.0).at(WINDOW.0 / 2.0, WINDOW.1 / 2.0));
    rt.frame(FRAME);
    drag(&mut rt, (512.0, 360.0), (712.0, 460.0));
    let expect = WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(100.0, 50.0));
    assert_eq!(kinds(&rt), [Kind::Rect(expect)]);
}

#[path = "interaction_tests.rs"]
mod selection;
