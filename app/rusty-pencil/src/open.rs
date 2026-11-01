//! Opening scene files into tabs, from a path on disk or from bytes the user picked or dropped.
use crate::{
    State,
    files::{identity, name},
    platform::{self, Picked},
};
use pen_core::file;
use rune::Tx;
use std::path::Path;

impl State {
    /// Opens a scene file from disk (`--open`, drops on native).
    pub(crate) fn open_path(&mut self, path: &Path, tx: &mut Tx) {
        let path = identity(path);
        if self.switch_to_open(&path, tx) {
            return;
        }
        match std::fs::read(&path) {
            Ok(bytes) => self.open_picked(&Picked { path, bytes }, tx),
            Err(error) => self.files.status.set(format!("Could not open: {error}")),
        }
    }

    /// Switches to the tab that already shows `path`; true when there was one, or when a tab is
    /// closing and nothing may open.
    fn switch_to_open(&mut self, path: &Path, tx: &mut Tx) -> bool {
        if self.tabs.closing.is_some() {
            return true;
        }
        let Some(id) = self.tabs.entries.iter().find_map(|t| {
            let files = t.parked.as_ref().map_or(&self.files, |c| &c.files);
            files
                .path
                .as_deref()
                .is_some_and(|p| platform::same_file(p, path))
                .then_some(t.id)
        }) else {
            return false;
        };
        self.switch_tab(id, tx);
        true
    }

    /// Opens a scene the user picked or dropped, in a new tab.
    pub(crate) fn open_picked(&mut self, picked: &Picked, tx: &mut Tx) {
        let path = identity(&picked.path);
        if self.switch_to_open(&path, tx) {
            return;
        }
        match file::load(&picked.bytes).map_err(|e| e.to_string()) {
            Ok(scene) => {
                self.new_tab(tx);
                self.install(&scene);
                self.files.path = Some(path.to_owned());
                self.files.dirty = false;
                self.files.refresh_label();
                self.files.changed(tx);
                self.autosave();
                self.files.status.set(if scene.skipped_elements > 0 {
                    format!(
                        "Opened {} · skipped {} unsupported elements",
                        name(&path),
                        scene.skipped_elements
                    )
                } else if scene.dropped_bindings > 0 {
                    format!(
                        "Opened {} · dropped {} invalid bindings",
                        name(&path),
                        scene.dropped_bindings
                    )
                } else {
                    format!("Opened {}", name(&path))
                });
            }
            Err(error) => self.files.status.set(format!("Could not open: {error}")),
        }
    }
}
