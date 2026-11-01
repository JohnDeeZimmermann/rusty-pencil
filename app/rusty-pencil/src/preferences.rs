//! Window-wide appearance is stored separately from scenes and workspace recovery.
use crate::{State, files};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub(crate) dark: bool,
}

impl State {
    pub(crate) fn restore_preferences(&mut self) {
        let Some(path) = self
            .tabs
            .workspace
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join("preferences.json"))
        else {
            return;
        };
        if !path.exists() {
            return;
        }
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
        {
            Ok(preferences) => self.preferences = preferences,
            Err(error) => self
                .files
                .status
                .set(format!("Could not restore preferences: {error}")),
        }
        self.sync_appearance();
    }

    pub(crate) fn save_preferences(&self) {
        let Some(path) = self
            .tabs
            .workspace
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join("preferences.json"))
        else {
            return;
        };
        let result = serde_json::to_vec(&self.preferences)
            .map_err(|e| e.to_string())
            .and_then(|bytes| files::atomic_write(&path, &bytes));
        if let Err(error) = result {
            self.files
                .status
                .set(format!("Could not save preferences: {error}"));
        }
    }

    /// The editor carries a transient appearance projection for rendering, never a file preference.
    pub(crate) fn sync_appearance(&mut self) {
        let mut settings = self.editor.settings();
        settings.dark = self.preferences.dark;
        self.editor
            .handle(pen_editor::InputEvent::Settings(settings));
    }
}

#[cfg(test)]
#[path = "preference_tests.rs"]
mod tests;
