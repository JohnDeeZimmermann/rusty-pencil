//! sRGB colours for the app chrome and test scene.
//!
//! `rune::Color` is linear-light sRGB, so a colour authored as `#1e1e1e` must be decoded, not
//! written into `Color`'s fields directly (that renders `#1e1e1e` as mid grey).

use rune::Color;

/// `0xRRGGBB` in sRGB to a linear, opaque `rune::Color`.
pub(crate) fn srgb(rgb: u32) -> Color {
    let channel = |shift: u32| decode(f32::from(((rgb >> shift) & 0xff) as u8) / 255.0);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: 1.0,
    }
}

fn decode(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Excalidraw's default stroke, `#1e1e1e`.
pub(crate) fn ink() -> Color {
    srgb(0x1e_1e_1e)
}
