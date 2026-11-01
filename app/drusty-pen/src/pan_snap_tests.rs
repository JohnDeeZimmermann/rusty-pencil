//! Pan and zoom move painted shapes rigidly: no pixel-snap hop when motion starts or settles.

use super::tests::{FRAME, app, drag, key};
use super::*;
use rune::{MagnifyEvent, Runtime, WheelEvent};

/// Painted instance rects, device px (the test env is 2×).
fn painted(rt: &Runtime<State>) -> Vec<[f32; 4]> {
    rt.instances.iter().map(|i| i.rect).collect()
}

/// The world origin on screen, device px.
fn world_origin(rt: &Runtime<State>) -> [f32; 2] {
    rt.state.camera.offset.get().map(|v| v * 2.0)
}

fn settle(rt: &mut Runtime<State>) {
    for _ in 0..120 {
        rt.frame(FRAME);
    }
}

#[test]
fn a_sub_pixel_pan_at_a_fractional_zoom_moves_every_shape_by_exactly_the_pan() {
    let mut rt = app();
    key(&mut rt, "r");
    drag(&mut rt, (400.0, 300.0), (520.0, 390.0));
    key(&mut rt, "\u{1b}");
    rt.magnify_event(MagnifyEvent::new(0.37).at(511.3, 357.7));
    settle(&mut rt);
    let (before, from) = (painted(&rt), world_origin(&rt));

    rt.wheel_event(WheelEvent::new(0.3, 0.3).at(300.0, 300.0));
    rt.frame(FRAME);
    let (moving, to) = (painted(&rt), world_origin(&rt));
    let d = [to[0] - from[0], to[1] - from[1]];
    assert!(d.iter().all(|v| v.fract().abs() > 0.1), "{d:?}");

    assert_eq!(before.len(), moving.len());
    let mut shapes = 0;
    for (a, b) in before.iter().zip(&moving) {
        if a.map(f32::to_bits) == b.map(f32::to_bits) {
            continue; // chrome
        }
        shapes += 1;
        let off = [
            b[0] - a[0] - d[0],
            b[1] - a[1] - d[1],
            b[2] - a[2],
            b[3] - a[3],
        ];
        // Rune places canvas paint on a 1/16 device-px grid: within 1/32 px of exact, where a
        // pixel-snap hop is up to half a pixel.
        assert!(
            off.iter().all(|o| o.abs() <= 1.0 / 32.0 + 1e-3),
            "{a:?} -> {b:?} is not a {d:?} shift"
        );
    }
    assert!(shapes > 0, "the rectangle moved");

    settle(&mut rt);
    assert_eq!(painted(&rt), moving, "nothing drifts once the pan stops");
}
