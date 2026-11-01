use crate::{Container, Font, Path, Props, TextAlign, WorldRect};
use serde::{Deserialize, Serialize};

/// Identifies one element for the life of its document. Ids are handed out in increasing
/// order and never reused, even after the element is removed.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId(u64);

impl ElementId {
    #[must_use]
    pub fn seed(self) -> u32 {
        seed_for(self)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }
}

/// One drawable thing on the canvas.
///
/// Geometry is unrotated; `angle` turns it clockwise about its bounds centre.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Element {
    id: ElementId,
    /// Bumped on every change, so renderers can cache geometry by `(id, version)`.
    version: u32,
    /// The only source of randomness for this element's rendering (hand-drawn style, M4).
    seed: u32,
    #[serde(default)]
    /// Clockwise radians about `kind.bounds().centre()`.
    pub angle: f32,
    #[serde(default)]
    /// Innermost first, outermost last.
    pub group_ids: Vec<crate::GroupId>,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub frame_id: Option<ElementId>,
    #[serde(default)]
    pub link: Option<Link>,
    #[serde(default)]
    /// Empty only on uncommitted elements; the document assigns it on insertion.
    pub z_key: crate::ZKey,
    #[serde(default)]
    pub bindings: [Option<crate::binding::Binding>; 2],
    pub props: Props,
    pub kind: Kind,
}

impl Element {
    /// A new element at version 1. The seed is derived from the id, so the same document
    /// always renders the same pixels.
    #[must_use]
    pub fn new(id: ElementId, kind: Kind, props: Props) -> Self {
        Self {
            id,
            version: 1,
            seed: seed_for(id),
            bindings: [None; 2],
            props,
            kind,
            angle: 0.0,
            group_ids: Vec::new(),
            locked: false,
            frame_id: None,
            link: None,
            z_key: crate::ZKey::default(),
        }
    }

    /// A geometry snapshot with identity and style preserved. Commands own version bumps.
    #[must_use]
    pub fn with_bounds(&self, bounds: WorldRect) -> Self {
        let mut changed = self.clone();
        changed.kind = self.kind.with_bounds(bounds);
        changed
    }

    /// Fresh identity for a pasted element; geometry and deterministic appearance survive.
    #[must_use]
    pub fn copied(&self, id: ElementId) -> Self {
        let mut out = self.clone();
        out.id = id;
        out.version = 1;
        out.z_key = crate::ZKey::default();
        out
    }

    // An unassigned key in a caller's pre-insertion snapshot means "keep the assigned key".
    pub(crate) fn same_content(&self, other: &Self) -> bool {
        self.id == other.id
            && self.kind == other.kind
            && self.bindings == other.bindings
            && self.props == other.props
            && self.seed == other.seed
            && self.angle.to_bits() == other.angle.to_bits()
            && self.group_ids == other.group_ids
            && self.locked == other.locked
            && self.frame_id == other.frame_id
            && self.link == other.link
            && (other.z_key.is_empty() || self.z_key == other.z_key)
    }

    pub(crate) fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }

    pub(crate) fn replace(&mut self, after: &Self) {
        self.kind.clone_from(&after.kind);
        self.props = after.props;
        self.bindings = after.bindings;
        self.angle = after.angle;
        self.group_ids.clone_from(&after.group_ids);
        self.locked = after.locked;
        self.frame_id = after.frame_id;
        self.link.clone_from(&after.link);
        if !after.z_key.is_empty() {
            self.z_key.clone_from(&after.z_key);
        }
        self.version = self.version.wrapping_add(1);
    }

    #[must_use]
    pub fn id(&self) -> ElementId {
        self.id
    }

    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }

    #[must_use]
    pub fn seed(&self) -> u32 {
        self.seed
    }
}

/// What an element is, with its geometry in world points.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Kind {
    Frame {
        name: String,
        bounds: WorldRect,
    },
    Image {
        file_id: crate::FileId,
        crop: crate::Crop,
        bounds: WorldRect,
    },
    Text {
        content: String,
        font: Font,
        size: f32,
        align: TextAlign,
        bounds: WorldRect,
        /// Set for text inside a shape or labelling a line; such text moves, re-fits and is
        /// deleted with its container.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        container: Option<Container>,
    },
    Rect(WorldRect),
    /// The rhombus inscribed in the rect: its corners are the rect's edge midpoints.
    Diamond(WorldRect),
    /// The ellipse inscribed in the rect.
    Ellipse(WorldRect),
    Line(Path),
    Arrow(Path),
    Freedraw(Path),
}

impl Kind {
    pub fn path_mut(&mut self) -> Option<&mut Path> {
        match self {
            Self::Line(p) | Self::Arrow(p) | Self::Freedraw(p) => Some(p),
            _ => None,
        }
    }

    #[must_use]
    pub fn with_bounds(&self, bounds: WorldRect) -> Self {
        match self {
            Self::Text {
                content,
                font,
                size,
                align,
                bounds: old,
                container,
            } => {
                let next_size = if old.height() > 0.0 {
                    size * bounds.height() / old.height()
                } else {
                    *size
                }
                .clamp(1.0, 1000.0);
                let scale = next_size / size;
                let p = bounds.min();
                Self::Text {
                    content: content.clone(),
                    font: *font,
                    size: next_size,
                    align: *align,
                    container: *container,
                    bounds: WorldRect::from_corners(
                        p,
                        crate::WorldPt::new(p.x + old.width() * scale, p.y + old.height() * scale),
                    ),
                }
            }
            Self::Frame { name, .. } => Self::Frame {
                name: name.clone(),
                bounds,
            },
            Self::Image { file_id, crop, .. } => Self::Image {
                file_id: file_id.clone(),
                crop: *crop,
                bounds,
            },
            Self::Rect(_) => Self::Rect(bounds),
            Self::Diamond(_) => Self::Diamond(bounds),
            Self::Ellipse(_) => Self::Ellipse(bounds),
            Self::Line(p) => Self::Line(p.with_bounds(bounds)),
            Self::Arrow(p) => Self::Arrow(p.with_bounds(bounds)),
            Self::Freedraw(p) => Self::Freedraw(p.with_bounds(bounds)),
        }
    }

    #[must_use]
    pub fn bounds(&self) -> WorldRect {
        match self {
            Kind::Frame { bounds: r, .. }
            | Kind::Image { bounds: r, .. }
            | Kind::Text { bounds: r, .. }
            | Kind::Rect(r)
            | Kind::Diamond(r)
            | Kind::Ellipse(r) => *r,
            Kind::Line(p) | Kind::Arrow(p) | Kind::Freedraw(p) => p.bounds(),
        }
    }

    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Line(p) | Self::Arrow(p) | Self::Freedraw(p) => Some(p),
            _ => None,
        }
    }

    /// The element this text belongs to, if it is bound text.
    #[must_use]
    pub fn container(&self) -> Option<Container> {
        match self {
            Self::Text { container, .. } => *container,
            _ => None,
        }
    }

    #[must_use]
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::Line(_) | Self::Arrow(_))
    }
}

/// Corner style for rectangles and diamonds. Ellipses ignore it.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Edges {
    Sharp,
    /// Excalidraw's adaptive radius: a quarter of the shorter side, at most 32 points.
    #[default]
    Round,
}

/// SplitMix64 of the id, folded to 32 bits: well spread even for consecutive ids.
fn seed_for(id: ElementId) -> u32 {
    let mut z = id.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z ^ (z >> 32)) as u32
}

/// Local element destinations and explicit web URLs survive scene round trips.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Web(String),
    Element(ElementId),
}
