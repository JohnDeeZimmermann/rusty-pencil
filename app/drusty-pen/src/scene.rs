//! The M0 test scene, in world points. Replaced by the document's draw list in M1.

use crate::palette::{ink, srgb};
use rune::Shape;

/// A rectangle straddling the world origin (so half of it has negative coordinates), and a
/// small cross at the origin.
pub(crate) fn test_scene() -> Shape {
    let (x0, y0, x1, y1) = (-160.0, -100.0, 160.0, 100.0);
    let rect = vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    // Excalidraw's light blue background and red, as sRGB.
    let (fill, axis) = (srgb(0xa5_d8_ff), srgb(0xe0_31_31));
    Shape::new()
        .fill(rect.clone(), fill)
        .stroke_with(rect, true, 2.0, rune::LineCap::Round, ink())
        .line([-12.0, 0.0], [12.0, 0.0], 1.0, rune::LineCap::Butt, axis)
        .line([0.0, -12.0], [0.0, 12.0], 1.0, rune::LineCap::Butt, axis)
}
