use std::collections::BTreeSet;

use pen_core::{Document, ElementId, WorldPt, WorldRect};

/// Stable id order, independent of document z-order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection(BTreeSet<ElementId>);

impl Selection {
    pub fn ids(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.0.iter().copied()
    }
    #[must_use]
    pub fn contains(&self, id: ElementId) -> bool {
        self.0.contains(&id)
    }
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
    pub(crate) fn insert(&mut self, id: ElementId) {
        self.0.insert(id);
    }
    pub(crate) fn toggle(&mut self, id: ElementId) {
        if !self.0.remove(&id) {
            self.0.insert(id);
        }
    }
    pub(crate) fn only(id: ElementId) -> Self {
        Self(BTreeSet::from([id]))
    }
    pub(crate) fn all(doc: &Document) -> Self {
        Self(doc.elements().iter().map(pen_core::Element::id).collect())
    }
    pub(crate) fn elements<'a>(
        &'a self,
        doc: &'a Document,
    ) -> impl Iterator<Item = &'a pen_core::Element> + 'a {
        self.ids().filter_map(|id| doc.get(id))
    }

    #[must_use]
    pub fn bounds(&self, doc: &Document) -> Option<WorldRect> {
        self.elements(doc)
            .map(|e| e.kind.bounds())
            .reduce(WorldRect::union)
    }
}

/// Screen-point dimensions shared by overlay rendering and handle hit testing.
pub const HANDLE_SIZE: f32 = 8.0;
pub const SELECTION_PADDING: f32 = 5.0;
const HANDLE_REACH: f32 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Nw,
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
}

impl Handle {
    pub const ALL: [Self; 8] = [
        Self::Nw,
        Self::N,
        Self::Ne,
        Self::E,
        Self::Se,
        Self::S,
        Self::Sw,
        Self::W,
    ];

    pub(crate) fn axes(self) -> (f32, f32) {
        match self {
            Self::Nw => (-1.0, -1.0),
            Self::N => (0.0, -1.0),
            Self::Ne => (1.0, -1.0),
            Self::E => (1.0, 0.0),
            Self::Se => (1.0, 1.0),
            Self::S => (0.0, 1.0),
            Self::Sw => (-1.0, 1.0),
            Self::W => (-1.0, 0.0),
        }
    }

    /// Handle centres are padded from the geometry by a constant screen distance.
    #[must_use]
    pub fn position(self, bounds: WorldRect, zoom: f32) -> WorldPt {
        let (x, y) = self.axes();
        let c = bounds.centre();
        let pad = SELECTION_PADDING / zoom;
        WorldPt::new(
            c.x + x * (bounds.width() / 2.0 + pad),
            c.y + y * (bounds.height() / 2.0 + pad),
        )
    }

    pub(crate) fn hit(bounds: WorldRect, point: WorldPt, zoom: f32) -> Option<Self> {
        Self::ALL.into_iter().find(|h| {
            let p = h.position(bounds, zoom);
            (p.x - point.x).abs() * zoom <= HANDLE_REACH
                && (p.y - point.y).abs() * zoom <= HANDLE_REACH
        })
    }
}
