use std::collections::BTreeMap;

use crate::{Element, ElementId};

/// The scene in bottom-to-top order. Mutated only through reversible commands.
#[derive(Debug, Clone, Default)]
pub struct Document {
    elements: Vec<Element>,
    files: BTreeMap<crate::FileId, crate::BinaryFile>,
    index: BTreeMap<ElementId, usize>,
    bound_arrows: BTreeMap<ElementId, Vec<ElementId>>,
    next_id: u64,
    revision: u64,
}

/// A reversible change. Groups validate atomically and undo in reverse order.
#[expect(
    clippy::large_enum_variant,
    reason = "Patches own two inline snapshots; boxing every history operation adds allocation"
)]
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    File {
        id: crate::FileId,
        file: Option<crate::BinaryFile>,
    },
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
    #[error("invalid embedded image, missing source or invalid crop")]
    Image,
    #[error("an element with id {0:?} already exists")]
    Duplicate(ElementId),
    #[error("patch does not match element {0:?}")]
    Conflict(ElementId),
}

impl Document {
    #[must_use]
    pub fn files(&self) -> &BTreeMap<crate::FileId, crate::BinaryFile> {
        &self.files
    }

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

    /// Derived back-references, refreshed atomically after each command group.
    #[must_use]
    pub fn bound_arrows(&self, shape: ElementId) -> &[ElementId] {
        self.bound_arrows.get(&shape).map_or(&[], Vec::as_slice)
    }

    /// Changes once per successful command, including a whole group.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the inverse. Element validation touches affected ids; file removal checks
    /// surviving references. Errors leave versions, revision and all contents unchanged.
    pub fn apply(&mut self, command: Command) -> Result<Command, DocError> {
        let mut staged = BTreeMap::new();
        self.validate(&command, &mut staged)?;
        let mut files = BTreeMap::new();
        file_changes(&command, &mut files);
        let valid_image = |e: &Element| match &e.kind {
            crate::Kind::Image { file_id, crop, .. } => {
                crop.valid()
                    && files
                        .get(file_id)
                        .copied()
                        .unwrap_or_else(|| self.files.contains_key(file_id))
            }
            _ => true,
        };
        if staged.values().flatten().any(|e| !valid_image(e))
            || (files.values().any(|present| !present)
                && self
                    .elements
                    .iter()
                    .filter(|e| !staged.contains_key(&e.id()))
                    .any(|e| !valid_image(e)))
        {
            return Err(DocError::Image);
        }
        let order_changed = changes_order(&command);
        let inverse = self.apply_validated(command)?;
        if order_changed {
            self.elements.sort_by(|a, b| a.z_key.cmp(&b.z_key));
            self.reindex();
        }
        self.bound_arrows.clear();
        for e in &self.elements {
            for b in e.bindings.iter().flatten() {
                let ids = self.bound_arrows.entry(b.element).or_default();
                if !ids.contains(&e.id()) {
                    ids.push(e.id());
                }
            }
        }
        debug_assert!(
            self.elements
                .iter()
                .all(|e| e.bindings.iter().flatten().all(|b| self
                    .bound_arrows
                    .get(&b.element)
                    .is_some_and(|ids| ids.contains(&e.id()))))
        );
        debug_assert!(
            self.bound_arrows
                .iter()
                .all(
                    |(shape, arrows)| arrows.iter().all(|id| self.get(*id).is_some_and(|e| e
                        .bindings
                        .iter()
                        .flatten()
                        .any(|b| b.element == *shape)))
                )
        );
        self.revision = self.revision.wrapping_add(1);
        Ok(inverse)
    }

    fn validate(
        &self,
        command: &Command,
        staged: &mut BTreeMap<ElementId, Option<Element>>,
    ) -> Result<(), DocError> {
        match command {
            Command::File { id, file } => {
                if file
                    .as_ref()
                    .is_some_and(|file| !file.valid() || &file.id() != id)
                {
                    return Err(DocError::Image);
                }
            }
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
            Command::File { id, file } => {
                let old = if let Some(file) = file {
                    self.files.insert(id.clone(), file)
                } else {
                    self.files.remove(&id)
                };
                Command::File { id, file: old }
            }
            Command::Group(commands) => {
                let mut inverses = Vec::with_capacity(commands.len());
                for command in commands {
                    inverses.push(self.apply_validated(command)?);
                }
                inverses.reverse();
                Command::Group(inverses)
            }
            Command::Add { mut element, at } => {
                let id = element.id();
                self.next_id = self.next_id.max(id.get());
                let at = at.min(self.elements.len());
                if element.z_key.is_empty() {
                    element.z_key = crate::ZKey::between(
                        at.checked_sub(1)
                            .and_then(|i| self.elements.get(i))
                            .map(|e| &e.z_key),
                        self.elements.get(at).map(|e| &e.z_key),
                    );
                }
                self.elements.insert(at, element);
                self.reindex();
                Command::Remove { id }
            }
            Command::Remove { id } => {
                let at = *self.index.get(&id).ok_or(DocError::Missing(id))?;
                let element = self.elements.remove(at);
                self.reindex();
                Command::Add { element, at }
            }
            Command::Patch {
                id,
                before: _,
                after,
            } => {
                let at = *self.index.get(&id).ok_or(DocError::Missing(id))?;
                let original = self.elements[at].clone();
                self.elements[at].replace(&after);
                Command::Patch {
                    id,
                    before: self.elements[at].clone(),
                    after: original,
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

fn changes_order(command: &Command) -> bool {
    match command {
        Command::File { .. } => false,
        Command::Group(commands) => commands.iter().any(changes_order),
        Command::Patch { before, after, .. } => {
            !after.z_key.is_empty() && before.z_key != after.z_key
        }
        Command::Add { .. } | Command::Remove { .. } => true,
    }
}

fn file_changes(command: &Command, out: &mut BTreeMap<crate::FileId, bool>) {
    match command {
        Command::File { id, file } => {
            out.insert(id.clone(), file.is_some());
        }
        Command::Group(commands) => {
            for command in commands {
                file_changes(command, out);
            }
        }
        _ => {}
    }
}
