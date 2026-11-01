//! Design tokens. `DESIGN.md` explains them; views use only these, never literals.

use pen_core::Srgba;
use rune::Color;
use rune::tok::TextStyle;

use crate::palette;

// Colour, as document colours so icons can go through the renderer. `c()` converts for rune.
pub(crate) const INK: Srgba = Srgba::rgb(0x1e_1e_1e);
pub(crate) const PAPER: Srgba = Srgba::rgb(0xff_ff_ff);
pub(crate) const CANVAS: Srgba = Srgba::rgb(0xf8_f9_f7);
pub(crate) const ACCENT: Srgba = Srgba::rgb(0xdd_ee_e8);
pub(crate) const ON_ACCENT: Srgba = Srgba::rgb(0x23_5e_4d);
pub(crate) const ACCENT_SOFT: Srgba = Srgba::rgb(0xf0_f3_f1);
pub(crate) const MUTED: Srgba = Srgba::rgb(0x70_78_74);
pub(crate) const BORDER: Srgba = Srgba::rgb(0xdd_e2_de);
pub(crate) const SHADOW: Srgba = Srgba { a: 18, ..INK };

pub(crate) fn c(color: Srgba) -> Color {
    palette::color(color)
}

// Shape.
pub(crate) const OUTLINE: f32 = 1.0;
pub(crate) const LIFT: f32 = 2.0;
pub(crate) const R_ISLAND: f32 = 12.0;
pub(crate) const R_BUTTON: f32 = 8.0;

// Spacing (4 pt grid).
pub(crate) const PADDING: f32 = 6.0;
pub(crate) const R_STATUS: f32 = 8.0;
pub(crate) const SHORTCUT: f32 = 10.0;
pub(crate) const SHORTCUT_INSET: f32 = 3.0;
pub(crate) const S1: f32 = 4.0;
pub(crate) const S2: f32 = 8.0;
pub(crate) const S3: f32 = 12.0;
pub(crate) const S4: f32 = 16.0;

// Sizes.
pub(crate) const BUTTON: f32 = 44.0;
pub(crate) const ICON: f32 = 20.0;
pub(crate) const ICON_STROKE: f32 = 1.7;
pub(crate) const PRESS: f32 = 0.98;

// Type.
pub(crate) const T_HINT: TextStyle = TextStyle::new(12.0, 450.0);
pub(crate) const T_LABEL: TextStyle = TextStyle::new(12.0, 500.0);
pub(crate) const T_BADGE: TextStyle = TextStyle::new(9.0, 500.0);

/// Transparent teal for the marquee; it never obscures document strokes.
pub(crate) const SELECTION_WASH: Srgba = Srgba { a: 24, ..ON_ACCENT };
pub(crate) const R_HANDLE: f32 = 2.0;

// M4 inspector, file status, and confirmation surfaces.
pub(crate) const INSPECTOR_TOP: f32 = 128.0 + TAB_BAR_HEIGHT;
/// Inner column is 192 pt: six 28 pt swatches with 4 pt gaps fit exactly.
pub(crate) const INSPECTOR_WIDTH: f32 = INSPECTOR_INNER + S3 * 2.0;
pub(crate) const INSPECTOR_INNER: f32 = 192.0;
pub(crate) const FILE_STATUS_WIDTH: f32 = 440.0;
pub(crate) const CONFIRM_WIDTH: f32 = 360.0;

/// Theme mapping is presentation-only: document colours always remain sRGB.
pub(crate) fn themed(color: Srgba, dark: bool) -> Color {
    c(if dark { color.dark() } else { color })
}
pub(crate) fn surface(el: rune::El, radius: f32, dark: bool) -> rune::El {
    el.radius(radius)
        .fill(themed(PAPER, dark))
        .border(OUTLINE, themed(BORDER, dark))
        .shadow(LIFT, 8.0, c(SHADOW))
}
pub(crate) fn kit(dark: bool) -> rune_kit::Kit {
    let mut kit = rune_kit::Kit::neutral(dark);
    kit.c.bg_base = themed(PAPER, dark);
    kit.c.text_base = themed(INK, dark);
    kit.c.text_muted = themed(MUTED, dark);
    kit.c.border_focus = themed(ON_ACCENT, dark);
    kit.c.layer_01 = themed(ACCENT_SOFT, dark);
    kit.ty.ui = T_LABEL;
    kit.ty.ui_medium = T_LABEL;
    kit
}
pub(crate) const PANEL_HEIGHT_MARGIN: f32 = 220.0 + TAB_BAR_HEIGHT;

// Inspector controls: denser than the toolbar, which stays the primary 44 pt surface.
/// Segmented-control track height; segments sit inside it with `SEGMENT_INSET`.
pub(crate) const CONTROL: f32 = 32.0;
pub(crate) const SEGMENT_INSET: f32 = 2.0;
pub(crate) const R_CONTROL: f32 = 6.0;
pub(crate) const SWATCH: f32 = 28.0;
pub(crate) const SWATCH_CHIP: f32 = 18.0;
pub(crate) const SLIDER_HEIGHT: f32 = 20.0;
pub(crate) const SWITCH_W: f32 = 28.0;
pub(crate) const SWITCH_H: f32 = 16.0;
/// Gap between a field label and its control.
pub(crate) const S_LABEL: f32 = 6.0;
/// Gap between inspector sections.
pub(crate) const S_SECTION: f32 = 14.0;
pub(crate) const T_CAPTION: TextStyle = TextStyle::new(11.0, 500.0);
pub(crate) const T_CONTROL: TextStyle = TextStyle::new(12.0, 450.0);
/// Symbol labels (arrowheads) need more size than words to read at a glance.
pub(crate) const T_GLYPH: TextStyle = TextStyle::new(16.0, 400.0);

// Arrange menu: two compact action columns, clamped to the viewport.
pub(crate) const MENU_COLUMN: f32 = 216.0;
pub(crate) const MENU_ROW: f32 = 30.0;
pub(crate) const MENU_WIDTH: f32 = 456.0;
pub(crate) const MENU_HEIGHT: f32 = 436.0;
pub(crate) const ROTATE_STEM: f32 = 19.0;

// Top-left application menu, separate from the centred drawing toolbar.
pub(crate) const APP_MENU_TOP: f32 = CHROME_TOP + CONTROL + PADDING * 2.0 + S2;
pub(crate) const FILE_TAB_WIDTH: f32 = 48.0;
pub(crate) const PREFERENCES_TAB_WIDTH: f32 = 96.0;

// Document tabs reserve a quiet strip above the floating drawing controls.
pub(crate) const TAB_BAR_HEIGHT: f32 = 48.0;
pub(crate) const TAB_WIDTH: f32 = 196.0;
pub(crate) const CHROME_TOP: f32 = TAB_BAR_HEIGHT + S4;
