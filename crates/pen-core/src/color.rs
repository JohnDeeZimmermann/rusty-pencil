use serde::{Deserialize, Serialize};
/// A colour as stored in the document: 8-bit sRGB (gamma-encoded, as CSS and Excalidraw store
/// it) with straight, not premultiplied, alpha. Renderers decode to linear light themselves.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Srgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Srgba {
    /// Inverts HSL lightness while preserving hue, saturation and alpha.
    #[must_use]
    pub fn dark(self) -> Self {
        let min = self.r.min(self.g).min(self.b);
        let max = self.r.max(self.g).max(self.b);
        let shift = 255 - i16::from(min) - i16::from(max);
        let channel = |v: u8| (i16::from(v) + shift).clamp(0, 255) as u8;
        Self {
            r: channel(self.r),
            g: channel(self.g),
            b: channel(self.b),
            a: self.a,
        }
    }

    #[must_use]
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    #[must_use]
    pub fn from_hex(input: &str) -> Option<Self> {
        let value = input.trim().strip_prefix('#').unwrap_or(input.trim());
        if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(value, 16).ok().map(Self::rgb)
    }

    /// Excalidraw's default stroke, `#1e1e1e`.
    pub const INK: Srgba = Srgba::rgb(0x1e_1e_1e);

    /// An opaque colour from `0xRRGGBB`.
    #[must_use]
    pub const fn rgb(rgb: u32) -> Self {
        Self {
            r: (rgb >> 16) as u8,
            g: (rgb >> 8) as u8,
            b: rgb as u8,
            a: 0xff,
        }
    }
}
