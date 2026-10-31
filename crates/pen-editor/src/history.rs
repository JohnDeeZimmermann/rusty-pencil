use crate::Selection;
use pen_core::{Command, DocError, Document};

#[derive(Debug, Clone)]
struct Entry {
    command: Command,
    before: Selection,
    after: Selection,
}

/// Commands on both stacks point toward the state that stack restores.
#[derive(Debug, Clone, Default)]
pub(crate) struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

impl History {
    pub(crate) fn execute(
        &mut self,
        doc: &mut Document,
        command: Command,
        before: Selection,
        after: Selection,
    ) -> Result<(), DocError> {
        let inverse = doc.apply(command)?;
        self.undo.push(Entry {
            command: inverse,
            before,
            after,
        });
        self.redo.clear();
        Ok(())
    }

    pub(crate) fn step(
        &mut self,
        doc: &mut Document,
        selection: &mut Selection,
        redo: bool,
    ) -> Result<bool, DocError> {
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        let Some(entry) = from.last() else {
            return Ok(false);
        };
        let inverse = doc.apply(entry.command.clone())?;
        *selection = if redo {
            entry.after.clone()
        } else {
            entry.before.clone()
        };
        if let Some(mut entry) = from.pop() {
            entry.command = inverse;
            to.push(entry);
        }
        Ok(true)
    }
}
