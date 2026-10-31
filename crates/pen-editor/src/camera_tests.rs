use crate::{Camera, Editor, InputEvent, ScreenPt, ScreenVec};
use pen_core::WorldPt;

fn close(a: WorldPt, b: WorldPt) -> bool {
    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3
}

#[test]
fn centred_camera_puts_the_world_point_mid_viewport() {
    let cam = Camera::centred_on(WorldPt::new(10.0, -20.0), ScreenVec::new(800.0, 600.0), 2.0);
    assert_eq!(
        cam.world_to_screen(WorldPt::new(10.0, -20.0)),
        ScreenPt::new(400.0, 300.0)
    );
}

#[test]
fn screen_and_world_round_trip() {
    let cam = Camera::centred_on(
        WorldPt::new(-500.0, 250.0),
        ScreenVec::new(1024.0, 720.0),
        3.5,
    );
    let p = WorldPt::new(-123.25, 987.5);
    assert!(close(cam.screen_to_world(cam.world_to_screen(p)), p));
}

#[test]
fn pan_moves_content_by_screen_points_at_any_zoom() {
    let mut editor = Editor::new(Camera::centred_on(
        WorldPt::ORIGIN,
        ScreenVec::new(800.0, 600.0),
        4.0,
    ));
    let before = editor.camera().world_to_screen(WorldPt::ORIGIN);
    editor.handle(InputEvent::Pan {
        by: ScreenVec::new(30.0, -10.0),
    });
    let after = editor.camera().world_to_screen(WorldPt::ORIGIN);
    assert_eq!((after.x - before.x, after.y - before.y), (30.0, -10.0));
}

#[test]
fn zoom_keeps_the_point_under_the_cursor_fixed() {
    let mut editor = Editor::new(Camera::centred_on(
        WorldPt::ORIGIN,
        ScreenVec::new(800.0, 600.0),
        1.0,
    ));
    let cursor = ScreenPt::new(612.0, 101.0);
    let under = editor.camera().screen_to_world(cursor);
    editor.handle(InputEvent::Zoom {
        about: cursor,
        factor: 1.7,
    });
    assert!((editor.camera().zoom() - 1.7).abs() < 1e-6);
    assert!(close(editor.camera().screen_to_world(cursor), under));
}

#[test]
fn zoom_is_clamped_and_bad_factors_are_ignored() {
    let mut cam = Camera::default();
    cam.zoom_about(ScreenPt::default(), 1000.0);
    assert!((cam.zoom() - Camera::MAX_ZOOM).abs() < f32::EPSILON);
    cam.zoom_about(ScreenPt::default(), 1e-6);
    assert!((cam.zoom() - Camera::MIN_ZOOM).abs() < f32::EPSILON);
    let before = cam;
    for bad in [0.0, -2.0, f32::NAN, f32::INFINITY] {
        cam.zoom_about(ScreenPt::new(5.0, 5.0), bad);
    }
    cam.pan_by(ScreenVec::new(f32::NAN, 1.0));
    assert_eq!(cam, before);
}
