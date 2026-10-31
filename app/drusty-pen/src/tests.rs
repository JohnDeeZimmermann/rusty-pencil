//! Headless app tests: the real view and handlers, driven through `rune::Runtime`.

use super::*;
use rune::{Env, Runtime};

fn app() -> Runtime<State> {
    let camera = Camera::centred_on(WorldPt::ORIGIN, ScreenVec::new(WINDOW.0, WINDOW.1), 1.0);
    let mut rt = Runtime::new(State::new(camera), view, WINDOW, Env::new(false, 2.0));
    rt.check_live_only(false);
    rt.frame(0.0);
    rt
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
