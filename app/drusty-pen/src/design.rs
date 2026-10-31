//! Design tokens. `DESIGN.md` explains them; views use only these, never literals.

use pen_core::Srgba;
use rune::Color;
use rune::tok::TextStyle;

use crate::palette;

// Colour, as document colours so icons can go through the renderer. `c()` converts for rune.
pub(crate) const INK: Srgba = Srgba::rgb(0x1e_1e_1e);
pub(crate) const PAPER: Srgba = Srgba::rgb(0xff_ff_ff);
pub(crate) const CANVAS: Srgba = Srgba::rgb(0xfd_fc_f8);
pub(crate) const ACCENT: Srgba = Srgba::rgb(0x7c_4d_ff);
pub(crate) const ON_ACCENT: Srgba = Srgba::rgb(0xff_ff_ff);
pub(crate) const ACCENT_SOFT: Srgba = Srgba::rgb(0xec_e5_ff);
pub(crate) const MUTED: Srgba = Srgba::rgb(0x6b_6b_75);
pub(crate) const BADGE: Srgba = Srgba::rgb(0xff_d8_4d);

pub(crate) fn c(color: Srgba) -> Color {
    palette::color(color)
}

// Shape.
pub(crate) const OUTLINE: f32 = 2.0;
pub(crate) const LIFT: f32 = 3.0;
pub(crate) const R_ISLAND: f32 = 16.0;
pub(crate) const R_BUTTON: f32 = 12.0;

// Spacing (4 pt grid).
pub(crate) const S1: f32 = 4.0;
pub(crate) const S3: f32 = 12.0;
pub(crate) const S4: f32 = 16.0;

// Sizes.
pub(crate) const BUTTON: f32 = 44.0;
pub(crate) const ICON: f32 = 22.0;
pub(crate) const ICON_STROKE: f32 = 2.2;
pub(crate) const PRESS: f32 = 0.9;

// Type.
pub(crate) const T_HINT: TextStyle = TextStyle::new(13.0, 500.0);
pub(crate) const T_LABEL: TextStyle = TextStyle::new(13.0, 650.0);
pub(crate) const T_BADGE: TextStyle = TextStyle::new(9.0, 750.0);

/// Raised chrome: ink outline, paper fill, hard ink shadow straight down.
pub(crate) fn raised(el: rune::El, radius: f32) -> rune::El {
    el.radius(radius)
        .fill(c(PAPER))
        .border(OUTLINE, c(INK))
        .shadow(LIFT, 0.0, c(INK))
}
