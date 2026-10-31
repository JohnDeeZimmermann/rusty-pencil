use crate::{Element, ElementId};

/// The scene: every element, bottom to top. The only source of truth for what is drawn.
///
/// Mutated only through [`Document::apply`], so every change has an inverse and can be undone.
#[derive(Debug, Clone, Default)]
pub struct Document {
    /// Z-order: index 0 is drawn first (at the bottom).
    elements: Vec<Element>,
    next_id: u64,
    revision: u64,
}

/// A reversible change to a [`Document`].
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Insert `element` at z-index `at` (clamped to the top).
    Add {
        element: Element,
        at: usize,
    },
    Remove {
        id: ElementId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocError {
    #[error("no element with id {0:?}")]
    Missing(ElementId),
    #[error("an element with id {0:?} already exists")]
    Duplicate(ElementId),
}

impl Document {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh id, never handed out before by this document.
    pub fn alloc_id(&mut self) -> ElementId {
        self.next_id += 1;
        ElementId::new(self.next_id)
    }

    /// Bottom to top.
    #[must_use]
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    /// Linear in the number of elements. Fine at M1 scale; index it once a profile says so.
    #[must_use]
    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.elements.iter().find(|e| e.id() == id)
    }

    /// Changes on every successful [`Document::apply`]. Lets views skip work when nothing
    /// changed; never stored in files.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Applies `command` and returns the command that undoes it. On error the document is
    /// unchanged.
    pub fn apply(&mut self, command: Command) -> Result<Command, DocError> {
        let inverse = match command {
            Command::Add { element, at } => {
                let id = element.id();
                if self.get(id).is_some() {
                    return Err(DocError::Duplicate(id));
                }
                self.next_id = self.next_id.max(id.get());
                self.elements.insert(at.min(self.elements.len()), element);
                Command::Remove { id }
            }
            Command::Remove { id } => {
                let at = self
                    .elements
                    .iter()
                    .position(|e| e.id() == id)
                    .ok_or(DocError::Missing(id))?;
                let element = self.elements.remove(at);
                Command::Add { element, at }
            }
        };
        self.revision += 1;
        Ok(inverse)
    }
}
