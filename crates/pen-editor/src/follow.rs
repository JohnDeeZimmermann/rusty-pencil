//! Bound text follows its container in live previews, on the same frame as the drag.
//!
//! Which texts follow is looked up once when the preview's membership or the document
//! changes; each pointer move only re-fits those few texts.
use pen_core::{Document, Element, ElementId, bound};

#[derive(Debug, Clone, Default)]
pub(crate) struct Followers {
    /// Document revision and preview ids the lookup was made for.
    key: Option<(u64, Vec<ElementId>)>,
    /// Committed bound text whose container is in the preview.
    texts: Vec<Element>,
    /// The base preview plus fitted followers; empty when nothing follows.
    composed: Vec<Element>,
}

impl Followers {
    pub(crate) fn update(&mut self, doc: &Document, base: &[Element]) {
        let fresh = self.key.as_ref().is_some_and(|(revision, ids)| {
            *revision == doc.revision() && ids.iter().copied().eq(base.iter().map(Element::id))
        });
        if !fresh {
            let ids: Vec<ElementId> = base.iter().map(Element::id).collect();
            self.texts.clear();
            if !ids.is_empty() {
                self.texts.extend(
                    doc.elements()
                        .iter()
                        .filter(|e| {
                            e.kind
                                .container()
                                .is_some_and(|c| ids.contains(&c.id()) && !ids.contains(&e.id()))
                        })
                        .cloned(),
                );
            }
            self.key = Some((doc.revision(), ids));
        }
        self.composed.clear();
        if self.texts.is_empty() {
            return;
        }
        self.composed.extend_from_slice(base);
        for text in &self.texts {
            let Some(link) = text.kind.container() else {
                continue;
            };
            if let Some(container) = base.iter().find(|e| e.id() == link.id()) {
                self.composed.push(bound::fit(text, container));
            }
        }
    }

    /// `None` when the base preview needs no additions.
    pub(crate) fn composed(&self) -> Option<&[Element]> {
        (!self.composed.is_empty()).then_some(self.composed.as_slice())
    }
}
