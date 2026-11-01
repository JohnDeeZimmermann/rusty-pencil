use std::collections::BTreeMap;

use crate::{Element, ElementId};

/// The scene in bottom-to-top order. Mutated only through reversible commands.
#[derive(Debug, Clone, Default)]
pub struct Document {
    elements: Vec<Element>,
    index: BTreeMap<ElementId, usize>,
    next_id: u64,
    revision: u64,
}

/// A reversible change. Groups validate atomically and undo in reverse order.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Add {
        element: Element,
        at: usize,
    },
    Remove {
        id: ElementId,
    },
    /// Snapshots must have the same identity; versions are managed by the document.
    Patch {
        id: ElementId,
        before: Element,
        after: Element,
    },
    Group(Vec<Command>),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocError {
    #[error("no element with id {0:?}")]
    Missing(ElementId),
    #[error("an element with id {0:?} already exists")]
    Duplicate(ElementId),
    #[error("patch does not match element {0:?}")]
    Conflict(ElementId),
}

impl Document {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh id, never reused even after removal or undo.
    pub fn alloc_id(&mut self) -> ElementId {
        self.next_id = self.next_id.saturating_add(1);
        ElementId::new(self.next_id)
    }

    /// Identity a new draft will receive on commit. This does not reserve the id.
    #[must_use]
    pub fn draft_id(&self) -> ElementId {
        ElementId::new(self.next_id.saturating_add(1))
    }

    /// Bottom to top.
    #[must_use]
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    #[must_use]
    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.index.get(&id).and_then(|at| self.elements.get(*at))
    }

    /// Changes once per successful command, including a whole group.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the inverse. Validation touches only affected ids; errors leave all state,
    /// including element versions and revision, unchanged.
    pub fn apply(&mut self, command: Command) -> Result<Command, DocError> {
        self.validate(&command, &mut BTreeMap::new())?;
        let inverse = self.apply_validated(command)?;
        self.revision = self.revision.wrapping_add(1);
        Ok(inverse)
    }

    fn validate(
        &self,
        command: &Command,
        staged: &mut BTreeMap<ElementId, Option<Element>>,
    ) -> Result<(), DocError> {
        match command {
            Command::Group(commands) => {
                for command in commands {
                    self.validate(command, staged)?;
                }
            }
            Command::Add { element, .. } => {
                let current = staged
                    .entry(element.id())
                    .or_insert_with(|| self.get(element.id()).cloned());
                if current.is_some() {
                    return Err(DocError::Duplicate(element.id()));
                }
                *current = Some(element.clone());
            }
            Command::Remove { id } => {
                let current = staged.entry(*id).or_insert_with(|| self.get(*id).cloned());
                if current.take().is_none() {
                    return Err(DocError::Missing(*id));
                }
            }
            Command::Patch { id, before, after } => {
                let current = staged.entry(*id).or_insert_with(|| self.get(*id).cloned());
                let element = current.as_mut().ok_or(DocError::Missing(*id))?;
                if after.id() != *id || !element.same_content(before) {
                    return Err(DocError::Conflict(*id));
                }
                element.replace(after);
            }
        }
        Ok(())
    }

    fn apply_validated(&mut self, command: Command) -> Result<Command, DocError> {
        Ok(match command {
            Command::Group(commands) => {
                let mut inverses = Vec::with_capacity(commands.len());
                for command in commands {
                    inverses.push(self.apply_validated(command)?);
                }
                inverses.reverse();
                Command::Group(inverses)
            }
            Command::Add { element, at } => {
                let id = element.id();
                self.next_id = self.next_id.max(id.get());
                self.elements.insert(at.min(self.elements.len()), element);
                self.reindex();
                Command::Remove { id }
            }
            Command::Remove { id } => {
                let at = *self.index.get(&id).ok_or(DocError::Missing(id))?;
                let element = self.elements.remove(at);
                self.reindex();
                Command::Add { element, at }
            }
            Command::Patch { id, before, after } => {
                let at = *self.index.get(&id).ok_or(DocError::Missing(id))?;
                self.elements[at].replace(&after);
                Command::Patch {
                    id,
                    before: after,
                    after: before,
                }
            }
        })
    }

    fn reindex(&mut self) {
        self.index.clear();
        self.index
            .extend(self.elements.iter().enumerate().map(|(at, e)| (e.id(), at)));
    }
}
