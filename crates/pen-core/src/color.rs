/// A colour as stored in the document: 8-bit sRGB (gamma-encoded, as CSS and Excalidraw store
/// it) with straight, not premultiplied, alpha. Renderers decode to linear light themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Srgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Srgba {
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
