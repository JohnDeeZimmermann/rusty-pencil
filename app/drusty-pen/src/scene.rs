//! Split cached paint around the moving elements once at gesture start. Static chunks are
//! memoized; only touched shapes rebuild during a drag, with document z-order preserved.
use crate::{layer, paint};
use pen_core::{Document, Element, ElementId};
use pen_render::{DrawList, SceneCache};
use rune::{El, Shape, el, memo};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
struct Part {
    moving: bool,
    shape: Shape,
    paint: DrawList,
}

#[derive(Debug, Default)]
pub(crate) struct Scene {
    parts: Vec<Part>,
    preview_ids: Vec<ElementId>,
    preview_parts: BTreeMap<ElementId, usize>,
    generation: u64,
}

impl Scene {
    pub(crate) fn show(
        &mut self,
        doc: &Document,
        cache: &SceneCache,
        preview: &[Element],
        committed: &Shape,
        changed: bool,
    ) {
        let same = self
            .preview_ids
            .iter()
            .copied()
            .eq(preview.iter().map(Element::id));
        if changed || !same || self.parts.is_empty() {
            self.generation = self.generation.wrapping_add(1);
            self.preview_ids.clear();
            self.preview_ids.extend(preview.iter().map(Element::id));
            self.preview_parts.clear();
            self.parts.clear();
            if preview.is_empty() {
                self.parts.push(Part {
                    moving: false,
                    shape: committed.clone(),
                    paint: DrawList::new(),
                });
            } else {
                let ids: BTreeSet<_> = self.preview_ids.iter().copied().collect();
                let mut scratch = DrawList::new();
                for element in doc.elements() {
                    if ids.contains(&element.id()) {
                        self.parts.push(Part {
                            moving: false,
                            shape: paint::shape(scratch.prims()),
                            paint: DrawList::new(),
                        });
                        scratch.clear();
                        self.preview_parts.insert(element.id(), self.parts.len());
                        self.parts.push(Part {
                            moving: true,
                            shape: Shape::new(),
                            paint: DrawList::new(),
                        });
                    } else if let Some(prims) = cache.element_prims(element.id()) {
                        for prim in prims {
                            scratch.push(prim.clone());
                        }
                    }
                }
                self.parts.push(Part {
                    moving: false,
                    shape: paint::shape(scratch.prims()),
                    paint: DrawList::new(),
                });
            }
        }
        for element in preview {
            if let Some(part) = self
                .preview_parts
                .get(&element.id())
                .and_then(|at| self.parts.get_mut(*at))
            {
                part.paint.clear();
                pen_render::draw(&element.kind, &element.props, &mut part.paint);
                part.shape = paint::shape(part.paint.prims());
            }
        }
    }

    pub(crate) fn view(&self) -> El {
        self.parts.iter().enumerate().fold(
            el().key("scene-parts").absolute(0.0, 0.0).size(0.0, 0.0),
            |node, (i, part)| {
                let key = format!("scene-part-{i}");
                node.child(if part.moving {
                    layer(part.shape.clone(), &key)
                } else {
                    memo(key.clone(), self.generation, || {
                        layer(part.shape.clone(), &key)
                    })
                })
            },
        )
    }
}
