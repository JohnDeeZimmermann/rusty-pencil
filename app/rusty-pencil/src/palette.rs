//! sRGB → linear conversion for rune. Chrome colours are tokens in `design.rs`.
//!
//! `rune::Color` is linear-light sRGB, so a colour authored as `#1e1e1e` must be decoded, not
//! written into `Color`'s fields directly (that renders `#1e1e1e` as mid grey).

use pen_core::Srgba;
use rune::Color;

/// `0xRRGGBB` in sRGB to a linear, opaque `rune::Color`.
#[cfg(test)]
pub(crate) fn srgb(rgb: u32) -> Color {
    srgba(rgb, 1.0)
}

/// `0xRRGGBB` in sRGB with straight `alpha` (0–1).
pub(crate) fn srgba(rgb: u32, alpha: f32) -> Color {
    let channel = |shift: u32| decode(f32::from(((rgb >> shift) & 0xff) as u8) / 255.0);
    Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: alpha,
    }
}

/// A document colour as rune paints it.
pub(crate) fn color(c: Srgba) -> Color {
    let rgb = (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b);
    srgba(rgb, f32::from(c.a) / 255.0)
}

fn decode(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
