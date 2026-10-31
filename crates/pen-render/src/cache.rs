use std::collections::BTreeMap;

use pen_core::{Document, ElementId};

use crate::{DrawList, Prim, draw};

/// Per-element paint, cached by `(id, version)`: rebuilding the scene after one element
/// changes re-flattens only that element.
#[derive(Debug, Default)]
pub struct SceneCache {
    entries: BTreeMap<ElementId, Entry>,
}

#[derive(Debug)]
struct Entry {
    version: u32,
    paint: DrawList,
}

impl SceneCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Brings the cache up to date with `doc`: builds paint for new or changed elements and
    /// drops paint for removed ones. Returns how many elements were (re)built.
    pub fn update(&mut self, doc: &Document) -> usize {
        let mut built = 0;
        for element in doc.elements() {
            let fresh = self
                .entries
                .get(&element.id())
                .is_some_and(|e| e.version == element.version());
            if !fresh {
                let mut paint = DrawList::new();
                draw(&element.kind, &element.props, &mut paint);
                self.entries.insert(
                    element.id(),
                    Entry {
                        version: element.version(),
                        paint,
                    },
                );
                built += 1;
            }
        }
        if self.entries.len() > doc.elements().len() {
            self.entries.retain(|id, _| doc.get(*id).is_some());
        }
        built
    }

    /// The scene's paint, bottom to top. Elements [`SceneCache::update`] hasn't seen are
    /// skipped.
    pub fn prims<'a>(&'a self, doc: &'a Document) -> impl Iterator<Item = &'a Prim> + 'a {
        doc.elements()
            .iter()
            .filter_map(|e| self.entries.get(&e.id()))
            .flat_map(|e| e.paint.prims())
    }

    /// Cached operations for one element; does not rebuild geometry.
    #[must_use]
    pub(crate) fn element_prims(&self, id: ElementId) -> Option<&[Prim]> {
        self.entries.get(&id).map(|entry| entry.paint.prims())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
