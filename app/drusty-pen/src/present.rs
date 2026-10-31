//! Presents a `pen_editor::Camera` through rune live values.

use pen_core::WorldPt;
use pen_editor::Camera;
use rune::{Live, LiveText};

/// Rune live values mirroring the editor camera. The world canvas is a zero-size node at the
/// viewport origin, so `live_scale` scales about (0, 0) and the presented transform is
/// `screen = world * zoom + offset`. With `screen = (world - scroll) * zoom` that makes
/// `offset = -scroll * zoom`.
pub(crate) struct LiveCamera {
    pub(crate) offset: Live<[f32; 2]>,
    pub(crate) scale: Live<f32>,
    pub(crate) zoom_label: LiveText,
}

impl LiveCamera {
    pub(crate) fn new(camera: &Camera) -> Self {
        let (offset, scale) = transform(camera);
        Self {
            offset: Live::new(offset),
            scale: Live::new(scale),
            zoom_label: LiveText::new(label(camera)),
        }
    }

    pub(crate) fn show(&self, camera: &Camera) {
        let (offset, scale) = transform(camera);
        self.offset.set(offset);
        self.scale.set(scale);
        self.zoom_label.set(label(camera));
    }
}

pub(crate) fn transform(camera: &Camera) -> ([f32; 2], f32) {
    let WorldPt { x, y } = camera.scroll();
    let z = camera.zoom();
    ([-x * z, -y * z], z)
}

fn label(camera: &Camera) -> String {
    format!("{:.0}%", camera.zoom() * 100.0)
}

/// `--camera X,Y,ZOOM`: open centred on world point (X, Y) at ZOOM. For screenshots and
/// manual checks; anything malformed is ignored.
pub(crate) fn camera_arg(args: &[String]) -> Option<Camera> {
    let at = args.iter().position(|a| a == "--camera")?;
    let parts: Vec<f32> = args
        .get(at + 1)?
        .split(',')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    let [x, y, zoom] = parts[..] else { return None };
    let viewport = pen_editor::ScreenVec::new(crate::WINDOW.0, crate::WINDOW.1);
    Some(Camera::centred_on(WorldPt::new(x, y), viewport, zoom))
}
