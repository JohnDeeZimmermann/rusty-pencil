//! Atomic workspace metadata references independent recovery files. Legacy single-canvas
//! recovery remains readable; screenshots never touch the real workspace.
use crate::{
    State, files,
    tabs::{Canvas, Tab, TabId},
};
use pen_core::file::{AppState, SceneFile};
use pen_editor::{Editor, InputEvent};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct Workspace {
    version: u32,
    active: TabId,
    next: u64,
    tabs: Vec<Entry>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    id: TabId,
    untitled: String,
    path: Option<PathBuf>,
    recovery: PathBuf,
    dirty: bool,
}

pub(crate) fn snapshot(editor: &Editor) -> SceneFile {
    SceneFile::new(
        editor.document(),
        AppState {
            settings: pen_core::Settings {
                dark: false,
                ..editor.settings()
            },
            scroll: editor.camera().scroll(),
            zoom: editor.camera().zoom(),
            style: editor.style(),
            font: editor.text_defaults().0,
            font_size: editor.text_defaults().1,
            text_align: editor.text_defaults().2,
        },
    )
}

impl State {
    pub(crate) fn persist_workspace(&self) -> bool {
        let Some(path) = &self.tabs.workspace else {
            return true;
        };
        let tabs = self
            .tabs
            .entries
            .iter()
            .filter_map(|tab| {
                let files = tab.parked.as_ref().map_or(&self.files, |c| &c.files);
                Some(Entry {
                    id: tab.id,
                    untitled: files.untitled.clone(),
                    path: files.path.clone(),
                    recovery: files.recovery.clone()?,
                    dirty: files.dirty,
                })
            })
            .collect();
        let workspace = Workspace {
            version: 1,
            active: self.tabs.active,
            next: self.tabs.next,
            tabs,
        };
        let result = serde_json::to_vec(&workspace)
            .map_err(|e| e.to_string())
            .and_then(|bytes| crate::platform::write(path, &bytes));
        if let Err(error) = result {
            self.files
                .status
                .set(format!("Could not save tabs: {error}"));
            return false;
        }
        true
    }

    pub(crate) fn restore_workspace(&mut self) -> bool {
        let Some(path) = &self.tabs.workspace else {
            return false;
        };
        if !crate::platform::exists(path) {
            return false;
        }
        let result = crate::platform::read(path).and_then(|bytes| {
            serde_json::from_slice::<Workspace>(&bytes).map_err(|e| e.to_string())
        });
        let workspace = match result {
            Ok(w) if w.version == 1 => w,
            _ => {
                self.files
                    .status
                    .set("Could not restore tabs; trying last canvas");
                return false;
            }
        };
        let mut entries = Vec::<Tab>::new();
        let mut failed = false;
        for entry in workspace.tabs {
            if entry.id.0 == 0
                || entry.id.0 >= u64::MAX - 1
                || entries.iter().any(|t| t.id == entry.id)
            {
                failed = true;
                continue;
            }
            match files::read(&entry.recovery) {
                Ok(scene) => {
                    let mut restored = State::new(*self.editor.camera());
                    restored.install(&scene);
                    restored.files.path = entry.path;
                    restored.files.recovery = Some(entry.recovery);
                    restored.files.dirty = entry.dirty;
                    restored.files.untitled = entry.untitled;
                    restored.files.refresh_label();
                    entries.push(Tab {
                        id: entry.id,
                        parked: Some(Canvas {
                            editor: restored.editor,
                            files: restored.files,
                            scene: restored.scene,
                        }),
                    });
                }
                Err(_) => failed = true,
            }
        }
        if entries.is_empty() {
            return false;
        }
        let at = entries
            .iter()
            .position(|t| t.id == workspace.active)
            .unwrap_or(0);
        let Some(canvas) = entries[at].parked.take() else {
            return false;
        };
        self.tabs.active = entries[at].id;
        self.tabs.next = entries
            .iter()
            .map(|t| t.id.0 + 1)
            .max()
            .unwrap_or(2)
            .max(workspace.next.min(u64::MAX - 1));
        self.tabs.entries = entries;
        let viewport = self.editor.viewport();
        self.editor = canvas.editor;
        self.editor.handle(InputEvent::Viewport(viewport));
        self.files = canvas.files;
        self.scene = canvas.scene;
        self.refresh_canvas();
        self.files.status.set(if failed {
            "Restored tabs; some recovery files could not be read"
        } else {
            "Restored open tabs"
        });
        true
    }
}
