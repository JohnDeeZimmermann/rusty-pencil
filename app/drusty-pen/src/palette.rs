//! Colours for the app chrome, and the document-colour adapter.
//!
//! `rune::Color` is linear-light sRGB, so a colour authored as `#1e1e1e` must be decoded, not
//! written into `Color`'s fields directly (that renders `#1e1e1e` as mid grey).

use pen_core::Srgba;
use rune::Color;

/// `0xRRGGBB` in sRGB to a linear, opaque `rune::Color`.
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

/// The chrome, after Excalidraw's light theme.
pub(crate) mod ui {
    use super::{srgb, srgba};
    use pen_core::Srgba;
    use rune::Color;

    pub(crate) fn island() -> Color {
        Color::WHITE
    }
    pub(crate) fn island_border() -> Color {
        srgba(0x00_00_00, 0.07)
    }
    pub(crate) fn shadow() -> Color {
        srgba(0x00_00_00, 0.08)
    }
    /// The selected tool's background.
    pub(crate) fn active() -> Color {
        srgb(0xe0_df_ff)
    }
    pub(crate) fn hover() -> Color {
        srgba(0x69_65_db, 0.08)
    }
    pub(crate) fn press() -> Color {
        srgba(0x69_65_db, 0.16)
    }
    /// Icon and caption inks are `Srgba` because icons go through the document renderer.
    pub(crate) const ICON_INK: Srgba = Srgba::rgb(0x1b_1b_1f);
    pub(crate) const ACTIVE_INK: Srgba = Srgba::rgb(0x30_2f_9c);
    pub(crate) const CAPTION: Srgba = Srgba::rgb(0x8a_8a_93);
    pub(crate) fn pill() -> Color {
        srgb(0xec_ec_f4)
    }
    pub(crate) fn pill_text() -> Color {
        srgb(0x33_33_3a)
    }
}
