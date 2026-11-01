//! Cached document paint and live replacements, composed in document z-order.
use crate::{DrawList, Prim, SceneCache, draw_element};
use pen_core::{Document, Element, ElementId, WorldPt};
use std::collections::{BTreeMap, BTreeSet};

/// A contiguous portion of scene paint. Its revision changes only when its paint changes;
/// adapters may retain backend resources until then. Revisions are unique within a scene.
#[derive(Debug)]
pub struct ScenePart {
    revision: u64,
    paint: DrawList,
    snapshot: Option<Element>,
    offset: WorldPt,
}

impl ScenePart {
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// World-point translation applied to the retained paint by the backend.
    #[must_use]
    pub fn offset(&self) -> WorldPt {
        self.offset
    }
    #[must_use]
    pub fn prims(&self) -> &[Prim] {
        self.paint.prims()
    }
}

/// Owns cache freshness and preview composition for one document's lifetime. Create a new
/// scene when replacing the document. Pan/zoom does not need to update this world-space paint.
#[derive(Debug, Default)]
pub struct Scene {
    cache: SceneCache,
    document_revision: Option<u64>,
    parts: Vec<ScenePart>,
    preview_ids: Vec<ElementId>,
    preview_parts: BTreeMap<ElementId, usize>,
    revision: u64,
}

impl Scene {
    /// Previews replace committed elements at their original z-position. A change to the
    /// document or preview membership rebuilds static partitions; pointer updates repaint
    /// only deformed previews; rigid previews update offsets without repainting.
    pub fn update(&mut self, doc: &Document, preview: &[Element]) {
        let changed = self.document_revision != Some(doc.revision());
        if changed {
            self.cache.update(doc);
            self.document_revision = Some(doc.revision());
        }
        let same = self
            .preview_ids
            .iter()
            .copied()
            .eq(preview.iter().map(Element::id));
        if changed || !same || self.parts.is_empty() {
            self.partition(doc, preview);
        }
        for element in preview {
            if let Some(at) = self.preview_parts.get(&element.id()).copied() {
                let part = &mut self.parts[at];
                if let Some(offset) = part
                    .snapshot
                    .as_ref()
                    .and_then(|before| crate::translation::translation(before, element))
                {
                    part.offset = offset;
                    continue;
                }
                self.revision = self.revision.wrapping_add(1);
                let part = &mut self.parts[at];
                part.paint.clear();
                draw_element(element, &mut part.paint);
                part.revision = self.revision;
                part.snapshot = Some(element.clone());
                part.offset = WorldPt::ORIGIN;
            }
        }
    }

    /// Bottom-to-top paint order, including empty partitions around previews.
    #[must_use]
    pub fn parts(&self) -> &[ScenePart] {
        &self.parts
    }

    fn partition(&mut self, doc: &Document, preview: &[Element]) {
        self.preview_ids.clear();
        self.preview_ids.extend(preview.iter().map(Element::id));
        self.preview_parts.clear();
        self.parts.clear();
        let ids: BTreeSet<_> = self.preview_ids.iter().copied().collect();
        let mut paint = DrawList::new();
        for element in doc.elements() {
            if ids.contains(&element.id()) {
                self.push_part(std::mem::take(&mut paint));
                self.preview_parts.insert(element.id(), self.parts.len());
                let mut retained = DrawList::new();
                if let Some(prims) = self.cache.element_prims(element.id()) {
                    for prim in prims {
                        retained.push(prim.clone());
                    }
                }
                self.push_part(retained);
                if let Some(part) = self.parts.last_mut() {
                    part.snapshot = Some(element.clone());
                }
            } else if let Some(prims) = self.cache.element_prims(element.id()) {
                for prim in prims {
                    paint.push(prim.clone());
                }
            }
        }
        self.push_part(paint);
        for element in preview.iter().filter(|e| doc.get(e.id()).is_none()) {
            self.preview_parts.insert(element.id(), self.parts.len());
            self.push_part(DrawList::new());
        }
    }

    fn push_part(&mut self, paint: DrawList) {
        self.revision = self.revision.wrapping_add(1);
        self.parts.push(ScenePart {
            revision: self.revision,
            paint,
            snapshot: None,
            offset: WorldPt::ORIGIN,
        });
    }
}
