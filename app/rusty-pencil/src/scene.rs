//! Rune adapter for composed scene paint. The renderer owns ordering and invalidation;
//! this adapter retains Rune shapes and memoized nodes by the renderer's part revisions.
//! Release, connected 7-element selection, 360 input+frame samples: mean 0.775 -> 0.214 ms,
//! p95 0.844 -> 0.239 ms. Live offsets avoid geometry, tessellation and UI rebuilds per move.
use pen_core::{Document, Element};
use rune::{El, Live, el, memo};

struct Part {
    revision: u64,
    offset: Live<[f32; 2]>,
    #[cfg(test)]
    empty: bool,
    node: El,
}

#[derive(Debug, Default)]
pub(crate) struct Scene {
    composition: pen_render::Scene,
    images: crate::images::Cache,
    document_revision: Option<u64>,
    parts: Vec<Part>,
    settings: pen_core::Settings,
}

impl Scene {
    pub(crate) fn show_with_settings(
        &mut self,
        doc: &Document,
        preview: &[Element],
        settings: pen_core::Settings,
    ) -> bool {
        let mut changed = false;
        if self.document_revision != Some(doc.revision()) {
            self.images.update(doc);
            self.document_revision = Some(doc.revision());
        }
        if self.settings.dark != settings.dark || self.settings.canvas != settings.canvas {
            self.parts.clear();
        }
        self.settings = settings;
        self.composition.update(doc, preview);
        let parts = self.composition.parts();
        changed |= self.parts.len() != parts.len();
        self.parts.truncate(parts.len());
        for (i, part) in parts.iter().enumerate() {
            let offset = part.offset();
            if let Some(old) = self.parts.get(i) {
                old.offset.set([offset.x, offset.y]);
            }
            if self
                .parts
                .get(i)
                .is_some_and(|old| old.revision == part.revision())
            {
                continue;
            }
            changed = true;
            let painted = Part {
                offset: Live::new([offset.x, offset.y]),
                revision: part.revision(),
                #[cfg(test)]
                empty: part.prims().is_empty(),
                node: crate::text::paint_with_images(part.prims(), settings, &self.images),
            };
            if let Some(old) = self.parts.get_mut(i) {
                *old = painted;
            } else {
                self.parts.push(painted);
            }
        }
        changed
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.parts.iter().all(|p| p.empty)
    }

    pub(crate) fn view(&self, canvas: crate::tabs::TabId) -> El {
        self.parts.iter().enumerate().fold(
            el().key("scene-parts").absolute(0.0, 0.0).size(0.0, 0.0),
            |node, (i, part)| {
                let key = format!("scene-part-{i}");
                node.child(
                    el().key(key.clone())
                        .absolute(0.0, 0.0)
                        .size(0.0, 0.0)
                        .live_offset(&part.offset)
                        .pixel_snap(false)
                        .child(memo(
                            format!("scene-paint-{i}"),
                            (
                                canvas,
                                part.revision,
                                self.settings.dark,
                                self.settings.canvas,
                            ),
                            || part.node.clone(),
                        )),
                )
            },
        )
    }
}

impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Part")
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}
