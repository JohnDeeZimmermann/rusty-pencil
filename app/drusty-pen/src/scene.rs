//! Rune adapter for composed scene paint. The renderer owns ordering and invalidation;
//! this adapter retains Rune shapes and memoized nodes by the renderer's part revisions.
use crate::{layer, paint};
use pen_core::{Document, Element};
use rune::{El, Shape, el, memo};

#[derive(Debug)]
struct Part {
    revision: u64,
    shape: Shape,
}

#[derive(Debug, Default)]
pub(crate) struct Scene {
    composition: pen_render::Scene,
    parts: Vec<Part>,
}

impl Scene {
    pub(crate) fn show(&mut self, doc: &Document, preview: &[Element]) {
        self.composition.update(doc, preview);
        let parts = self.composition.parts();
        self.parts.truncate(parts.len());
        for (i, part) in parts.iter().enumerate() {
            if self
                .parts
                .get(i)
                .is_some_and(|old| old.revision == part.revision())
            {
                continue;
            }
            let painted = Part {
                revision: part.revision(),
                shape: paint::shape(part.prims()),
            };
            if let Some(old) = self.parts.get_mut(i) {
                *old = painted;
            } else {
                self.parts.push(painted);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.parts.iter().all(|p| p.shape.is_empty())
    }

    pub(crate) fn view(&self) -> El {
        self.parts.iter().enumerate().fold(
            el().key("scene-parts").absolute(0.0, 0.0).size(0.0, 0.0),
            |node, (i, part)| {
                let key = format!("scene-part-{i}");
                node.child(memo(key.clone(), part.revision, || {
                    layer(part.shape.clone(), &key)
                }))
            },
        )
    }
}
