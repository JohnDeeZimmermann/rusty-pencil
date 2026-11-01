//! Rune adapter for composed scene paint. The renderer owns ordering and invalidation;
//! this adapter retains Rune shapes and memoized nodes by the renderer's part revisions.
use pen_core::{Document, Element};
use rune::{El, el, memo};

struct Part {
    revision: u64,
    #[cfg(test)]
    empty: bool,
    node: El,
}

#[derive(Debug, Default)]
pub(crate) struct Scene {
    composition: pen_render::Scene,
    parts: Vec<Part>,
    settings: pen_core::Settings,
}

impl Scene {
    pub(crate) fn show_with_settings(
        &mut self,
        doc: &Document,
        preview: &[Element],
        settings: pen_core::Settings,
    ) {
        if self.settings.dark != settings.dark || self.settings.canvas != settings.canvas {
            self.parts.clear();
        }
        self.settings = settings;
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
                #[cfg(test)]
                empty: part.prims().is_empty(),
                node: crate::text::paint_themed(part.prims(), settings),
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
        self.parts.iter().all(|p| p.empty)
    }

    pub(crate) fn view(&self) -> El {
        self.parts.iter().enumerate().fold(
            el().key("scene-parts").absolute(0.0, 0.0).size(0.0, 0.0),
            |node, (i, part)| {
                let key = format!("scene-part-{i}");
                node.child(memo(
                    key.clone(),
                    (part.revision, self.settings.dark, self.settings.canvas),
                    || part.node.clone(),
                ))
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
