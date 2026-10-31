use crate::{Srgba, WorldRect};

/// Identifies one element for the life of its document. Ids are handed out in increasing
/// order and never reused, even after the element is removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId(u64);

impl ElementId {
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
/// Z-order is not stored here: it is the element's position in its [`crate::Document`].
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    id: ElementId,
    /// Bumped on every change, so renderers can cache geometry by `(id, version)`.
    version: u32,
    /// The only source of randomness for this element's rendering (hand-drawn style, M4).
    seed: u32,
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
            props,
            kind,
        }
    }

    /// A geometry snapshot with identity and style preserved. Commands own version bumps.
    #[must_use]
    pub fn with_bounds(&self, bounds: WorldRect) -> Self {
        let mut changed = self.clone();
        changed.kind = self.kind.with_bounds(bounds);
        changed
    }

    pub(crate) fn same_content(&self, other: &Self) -> bool {
        self.id == other.id
            && self.kind == other.kind
            && self.props == other.props
            && self.seed == other.seed
    }

    pub(crate) fn replace(&mut self, after: &Self) {
        self.kind = after.kind;
        self.props = after.props;
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Rect(WorldRect),
    /// The rhombus inscribed in the rect: its corners are the rect's edge midpoints.
    Diamond(WorldRect),
    /// The ellipse inscribed in the rect.
    Ellipse(WorldRect),
}

impl Kind {
    #[must_use]
    pub fn with_bounds(self, bounds: WorldRect) -> Self {
        match self {
            Self::Rect(_) => Self::Rect(bounds),
            Self::Diamond(_) => Self::Diamond(bounds),
            Self::Ellipse(_) => Self::Ellipse(bounds),
        }
    }

    #[must_use]
    pub fn bounds(&self) -> WorldRect {
        match *self {
            Kind::Rect(r) | Kind::Diamond(r) | Kind::Ellipse(r) => r,
        }
    }
}

/// How an element is painted. Defaults match Excalidraw's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Props {
    pub stroke: Srgba,
    /// `None` is a transparent background.
    pub background: Option<Srgba>,
    /// In world points, so it scales with zoom.
    pub stroke_width: f32,
    pub edges: Edges,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            stroke: Srgba::INK,
            background: None,
            stroke_width: 2.0,
            edges: Edges::Round,
        }
    }
}

/// Corner style for rectangles and diamonds. Ellipses ignore it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
