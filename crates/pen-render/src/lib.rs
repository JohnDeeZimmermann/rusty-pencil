//! Turns a `pen-core` document plus the `pen-editor` draft into a backend-neutral draw list.
//!
//! Output is polylines in world points; the app presents them through the camera without
//! rebuilding them. Curves are flattened here, finely enough to stay smooth at the editor's
//! maximum zoom, so the result is camera-independent and cacheable.
#![forbid(unsafe_code)]

mod arrowheads;
mod cache;
pub mod export;
mod overlay;
mod path;
mod rough;
mod scene;
mod style;
mod translation;
pub use rough::{draw_element, draw_seeded};

use pen_core::{Kind, Props, Srgba, WorldPt};

pub use cache::SceneCache;
pub use overlay::{OverlayKind, OverlayRect, overlay};
pub use path::freehand_outline;
pub use scene::{Scene, ScenePart};

/// Flattening tolerance in world points: curves stay within this of their true position.
/// 0.02 pt is 0.6 px at the 3000% maximum zoom.
pub use pen_core::TOLERANCE;

/// One paint operation, in world points.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Image {
        file_id: pen_core::FileId,
        crop: pen_core::Crop,
        bounds: pen_core::WorldRect,
        angle: f32,
        opacity: u8,
    },
    Text {
        angle: f32,
        content: String,
        font: pen_core::Font,
        size: f32,
        align: pen_core::TextAlign,
        bounds: pen_core::WorldRect,
        color: Srgba,
        fit: TextFit,
        vertical_align: pen_core::VerticalAlign,
    },
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

/// How a text primitive sits in its bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFit {
    /// The bounds are the measured text.
    Free,
    /// Wraps at the bounds' width and is centred vertically in them (text inside a shape).
    Wrap,
    /// The bounds are the measured text, on a canvas-coloured backdrop that masks the line
    /// it labels.
    Label,
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
    let curved;
    let kind = if let Kind::Arrow(path) = kind
        && props.arrow_type == pen_core::ArrowType::Curved
    {
        curved = Kind::Arrow(pen_core::routing::curved(path));
        &curved
    } else {
        kind
    };
    let first = out.prims.len();
    draw_base(kind, props, out);
    if matches!(kind, Kind::Freedraw(_)) {
        let mut solid = *props;
        solid.fill = pen_core::FillStyle::Solid;
        style::finish(out, first, &solid);
    } else {
        style::finish(out, first, props);
    }
    if matches!(kind, Kind::Arrow(_))
        && props.stroke_width > 0.0
        && let Some(path) = kind.path()
    {
        arrowheads::draw(path, props, out);
    }
}

fn draw_base(kind: &Kind, props: &Props, out: &mut DrawList) {
    if let Kind::Image {
        file_id,
        crop,
        bounds,
    } = kind
    {
        out.push(Prim::Image {
            file_id: file_id.clone(),
            crop: *crop,
            bounds: *bounds,
            angle: 0.0,
            opacity: props.opacity,
        });
        return;
    }
    if let Kind::Text {
        content,
        font,
        size,
        align,
        bounds,
        container,
    } = kind
    {
        if !content.is_empty() {
            out.push(Prim::Text {
                angle: 0.0,
                vertical_align: props.vertical_align,
                content: content.clone(),
                font: *font,
                size: *size,
                align: *align,
                bounds: *bounds,
                color: props.stroke,
                fit: match container {
                    None => TextFit::Free,
                    Some(pen_core::Container::Shape(_)) => TextFit::Wrap,
                    Some(pen_core::Container::Path(_)) => TextFit::Label,
                },
            });
        }
        return;
    }
    if let Some(path) = kind.path() {
        path::draw_path(kind, path, props, out);
        return;
    }
    let outline = pen_core::outline(kind, props.edges);
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

#[cfg(test)]
mod overlay_tests;

#[cfg(test)]
mod scene_tests;

#[cfg(test)]
mod path_tests;

#[cfg(test)]
mod style_tests;

#[cfg(test)]
mod translation_tests;

#[cfg(test)]
mod export_tests;
