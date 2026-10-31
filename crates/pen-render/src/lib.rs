//! Turns a `pen-core` document plus the `pen-editor` draft into a backend-neutral draw list.
//!
//! Output is polylines in world points; the app presents them through the camera without
//! rebuilding them. Curves are flattened here, finely enough to stay smooth at the editor's
//! maximum zoom, so the result is camera-independent and cacheable.
#![forbid(unsafe_code)]

mod cache;
mod shapes;

use pen_core::{Kind, Props, Srgba, WorldPt};

pub use cache::SceneCache;

/// Flattening tolerance in world points: curves stay within this of their true position.
/// 0.02 pt is 0.6 px at the 3000% maximum zoom.
pub const TOLERANCE: f32 = 0.02;

/// One paint operation, in world points.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    /// A simple polygon filled with `color`.
    Fill { points: Vec<WorldPt>, color: Srgba },
    /// A polyline `width` world points wide, with round joins and caps; `closed` joins the
    /// last point back to the first.
    Stroke {
        points: Vec<WorldPt>,
        closed: bool,
        width: f32,
        color: Srgba,
    },
}

/// Paint operations in order: later ones draw on top. Reuse one across frames with
/// [`DrawList::clear`] to keep its allocation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawList {
    prims: Vec<Prim>,
}

impl DrawList {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn prims(&self) -> &[Prim] {
        &self.prims
    }

    pub fn clear(&mut self) {
        self.prims.clear();
    }

    pub fn push(&mut self, prim: Prim) {
        self.prims.push(prim);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prims.is_empty()
    }
}

/// Appends the paint for one element (or draft) to `out`: background fill, then stroke.
pub fn draw(kind: &Kind, props: &Props, out: &mut DrawList) {
    let outline = shapes::outline(kind, props.edges);
    if outline.len() < 2 {
        return;
    }
    if let Some(color) = props.background
        && !kind.bounds().is_degenerate()
    {
        out.push(Prim::Fill {
            points: outline.clone(),
            color,
        });
    }
    if props.stroke_width > 0.0 {
        out.push(Prim::Stroke {
            points: outline,
            closed: true,
            width: props.stroke_width,
            color: props.stroke,
        });
    }
}

#[cfg(test)]
mod tests;
