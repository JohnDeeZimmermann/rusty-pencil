//! Local scene files and one-second debounced recovery. Scripted screenshots are isolated
//! from the user's recovery unless they explicitly supply `--data-dir`.
use crate::{State, design as d, scene};
use pen_core::{
    Document, WorldPt,
    file::{self, AppState, SceneFile},
};
use pen_editor::{Camera, InputEvent};
use rune::{El, FileDialogOptions, Key, KeyEvent, Tx, col, el, row, text};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug)]
pub(crate) struct Files {
    pub(crate) path: Option<PathBuf>,
    pub(crate) recovery: Option<PathBuf>,
    pending: bool,
    deadline: Option<f64>,
    pub(crate) status: Status,
    confirm_reset: bool,
    pub(crate) dirty: bool,
    pub(crate) untitled: String,
    pub(crate) label: rune::LiveText,
}
impl Default for Files {
    fn default() -> Self {
        Self {
            path: None,
            recovery: None,
            pending: false,
            deadline: None,
            status: Status::default(),
            confirm_reset: false,
            dirty: false,
            untitled: "Untitled 1".into(),
            label: rune::LiveText::new("Untitled 1"),
        }
    }
}
#[derive(Debug)]
pub(crate) struct Status(rune::LiveText);
impl Default for Status {
    fn default() -> Self {
        Self(rune::LiveText::new("Local canvas"))
    }
}
impl Status {
    pub(crate) fn set(&self, value: impl Into<std::sync::Arc<str>>) {
        self.0.set(value);
    }
}

impl Files {
    pub(crate) fn title(&self) -> String {
        self.path
            .as_deref()
            .map_or_else(|| self.untitled.clone(), name)
    }
    pub(crate) fn refresh_label(&self) {
        self.label.set(format!(
            "{}{}",
            if self.dirty { "•  " } else { "" },
            self.title()
        ));
    }
    pub(crate) fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            self.refresh_label();
        }
    }

    pub(crate) fn changed(&mut self, tx: &mut Tx) {
        self.pending = true;
        self.deadline = None;
        self.status.set("Unsaved changes");
        tx.wake_after(Duration::from_secs(1));
    }
}

impl State {
    pub(crate) fn snapshot(&self) -> SceneFile {
        let mut scene = crate::workspace::snapshot(&self.editor);
        if let Some(session) = self.editor.text_session() {
            let element = &session.element;
            let empty = matches!(&element.kind, pen_core::Kind::Text { content, .. } if content.trim().is_empty());
            if let Some(at) = scene.elements.iter().position(|e| e.id() == element.id()) {
                if empty {
                    scene.elements.remove(at);
                } else {
                    scene.elements[at] = element.clone();
                }
            } else if !empty {
                scene.elements.push(element.clone());
            }
        }
        scene
    }

    pub(crate) fn restore(&mut self) {
        let args = rune::extra_args();
        let directory = args
            .iter()
            .position(|a| a == "--data-dir")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from);
        let isolated = std::env::args()
            .into_iter()
            .any(|a| matches!(a.as_str(), "--snap" | "--script" | "--tree"));
        self.files.recovery = directory
            .or_else(|| {
                if isolated {
                    None
                } else {
                    directories::ProjectDirs::from("", "", "drusty-pen")
                        .map(|p| p.data_local_dir().to_owned())
                }
            })
            .map(|p| p.join("autosave.drusty-pen"));
        self.files.status.set("Local canvas");
        self.tabs.workspace = self
            .files
            .recovery
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join("workspace.json"));
        self.restore_preferences();
        let restored_workspace = self.restore_workspace();
        if !restored_workspace
            && let Some(path) = self.files.recovery.clone().filter(|p| p.exists())
        {
            match read(&path) {
                Ok(scene) => {
                    self.install(&scene);
                    self.files.mark_dirty();
                    self.files.status.set("Restored last canvas");
                }
                Err(error) => self.files.status.set(format!("Could not restore: {error}")),
            }
        }
        if let Some(path) = args
            .iter()
            .position(|a| a == "--open")
            .and_then(|i| args.get(i + 1))
        {
            self.open_path(Path::new(path), &mut Tx::default());
        }
    }

    pub(crate) fn install(&mut self, scene_file: &SceneFile) {
        let Ok(doc) = scene_file.document() else {
            return;
        };
        if scene_file.dropped_bindings > 0 {
            eprintln!(
                "Dropped {} dangling or invalid bindings while opening scene",
                scene_file.dropped_bindings
            );
        }
        if scene_file.skipped_elements > 0 {
            self.files.status.set(format!(
                "Skipped {} unsupported elements",
                scene_file.skipped_elements
            ));
        }
        let app = &scene_file.app_state;
        self.editor
            .replace_document(doc, Camera::new(app.scroll, app.zoom), app.style);
        self.editor.handle(InputEvent::Settings(app.settings));
        self.editor
            .restore_text_defaults(app.font, app.font_size, app.text_align);
        self.scene = scene::Scene::default();
        self.refresh_canvas();
    }

    pub(crate) fn open_path(&mut self, path: &Path, tx: &mut Tx) {
        if self.tabs.closing.is_some() {
            return;
        }
        let path = identity(path);
        if let Some(id) = self.tabs.entries.iter().find_map(|t| {
            let files = t.parked.as_ref().map_or(&self.files, |c| &c.files);
            (files.path.as_deref() == Some(path.as_path())).then_some(t.id)
        }) {
            self.switch_tab(id, tx);
            return;
        }
        match read(&path) {
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

    fn save_tab_path(&mut self, id: crate::tabs::TabId, path: &Path, tx: &mut Tx) {
        let path = identity(path);
        if self.tabs.entries.iter().any(|t| {
            let files = t.parked.as_ref().map_or(&self.files, |c| &c.files);
            t.id != id && files.path.as_deref() == Some(path.as_path())
        }) {
            self.files
                .status
                .set("This file is already open in another tab");
            return;
        }
        let snapshot = if id == self.tabs.active {
            self.snapshot()
        } else if let Some(c) = self
            .tabs
            .entries
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.parked.as_ref())
        {
            crate::workspace::snapshot(&c.editor)
        } else {
            return;
        };
        match write(&path, &snapshot) {
            Ok(()) => {
                let files = if id == self.tabs.active {
                    &mut self.files
                } else if let Some(c) = self
                    .tabs
                    .entries
                    .iter_mut()
                    .find(|t| t.id == id)
                    .and_then(|t| t.parked.as_mut())
                {
                    &mut c.files
                } else {
                    return;
                };
                files.path = Some(path.to_owned());
                files.dirty = false;
                files.refresh_label();
                files.status.set(format!("Saved {}", name(&path)));
                if let Some(recovery) = &files.recovery {
                    match write(recovery, &snapshot) {
                        Ok(()) => files.pending = false,
                        Err(error) => files
                            .status
                            .set(format!("File saved; could not update recovery: {error}")),
                    }
                }
                self.persist_workspace();
                if self.tabs.closing == Some(id) {
                    self.close_tab(id, tx);
                }
            }
            Err(error) => self.files.status.set(format!("Could not save: {error}")),
        }
    }

    pub(crate) fn save_dialog(&mut self, save_as: bool, tx: &mut Tx) {
        if self.editor.text_session().is_some() {
            self.sync_text(tx);
            self.input(InputEvent::FinishText, tx);
        }
        let id = self.tabs.active;
        if !save_as && let Some(path) = self.files.path.clone() {
            self.save_tab_path(id, &path, tx);
            return;
        }
        let options = FileDialogOptions {
            title: "Save canvas".into(),
            file_name: Some(
                self.files
                    .path
                    .as_deref()
                    .map_or_else(|| "Untitled.drusty-pen".into(), name),
            ),
            directory: self
                .files
                .path
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_owned),
            ..Default::default()
        }
        .filter("drusty-pen scene", &["drusty-pen", "json", "excalidraw"]);
        tx.save_file(options, move |s: &mut State, result, tx| match result {
            Ok(Some(path)) => s.save_tab_path(id, &path, tx),
            Ok(None) => {
                s.tabs.closing = None;
            }
            Err(e) => s.files.status.set(format!("Could not save: {e}")),
        });
    }

    fn open_dialog(&mut self, tx: &mut Tx) {
        if self.editor.text_session().is_some() {
            self.sync_text(tx);
            self.input(InputEvent::FinishText, tx);
        }
        tx.open_file(
            FileDialogOptions {
                title: "Open canvas".into(),
                ..Default::default()
            }
            .filter("drusty-pen scene", &["drusty-pen", "json", "excalidraw"]),
            |s: &mut State, result, tx| match result {
                Ok(Some(path)) => s.open_path(&path, tx),
                Ok(None) => {}
                Err(e) => s.files.status.set(format!("Could not open: {e}")),
            },
        );
    }

    pub(crate) fn autosave(&mut self) -> bool {
        if !self.files.pending {
            return false;
        }
        let Some(path) = &self.files.recovery else {
            self.files.pending = false;
            return false;
        };
        match write(path, &self.snapshot()) {
            Ok(()) => {
                self.files.pending = false;
                self.files
                    .status
                    .set(self.files.path.as_deref().map_or_else(
                        || "Saved locally".into(),
                        |p| format!("{} · saved locally", name(p)),
                    ));
                self.persist_workspace();
            }
            Err(error) => {
                // Keep the pending scene so closing can retry. Avoid a hot retry loop.
                self.files
                    .status
                    .set(format!("Could not autosave: {error}"));
            }
        }
        true
    }
}

fn identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        match (parent.canonicalize(), path.file_name()) {
            (Ok(parent), Some(name)) => parent.join(name),
            _ => path.to_owned(),
        }
    })
}

pub(crate) fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn read(path: &Path) -> Result<SceneFile, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    file::load(&bytes).map_err(|e| e.to_string())
}

pub(crate) fn write(path: &Path, scene: &SceneFile) -> Result<(), String> {
    let bytes = file::save(scene).map_err(|e| e.to_string())?;
    atomic_write(path, &bytes)
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    if s.editor.text_session().is_some() && !s.text_edit.focused {
        s.sync_text(tx);
        s.input(InputEvent::FinishText, tx);
        return true;
    }
    if !s.files.pending {
        return false;
    }
    let deadline = *s.files.deadline.get_or_insert(now + 1.0);
    if !deadline.is_finite() {
        return false;
    }
    if now < deadline {
        tx.wake_after(Duration::from_secs_f64(deadline - now));
        return false;
    }
    // Clear the deadline after failure too: a new edit or close retries it.
    s.files.deadline = Some(f64::INFINITY);
    s.autosave()
}

pub(crate) fn close(s: &mut State, tx: &mut Tx) {
    s.sync_text(tx);
    s.input(InputEvent::FinishText, tx);
    if s.files.pending {
        s.autosave();
    }
    for tab in &mut s.tabs.entries {
        if let Some(canvas) = &mut tab.parked
            && canvas.files.pending
            && let Some(path) = &canvas.files.recovery
        {
            match write(path, &crate::workspace::snapshot(&canvas.editor)) {
                Ok(()) => canvas.files.pending = false,
                Err(error) => s.files.status.set(format!(
                    "Could not autosave {}: {error}",
                    canvas.files.title()
                )),
            }
        }
    }
    s.persist_workspace();
}

pub(crate) fn key(s: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if !event.mods.cmd || event.mods.alt {
        return false;
    }
    let c = match &event.key {
        Key::Command(c) => *c,
        Key::Text(t) if t.len() == 1 => t.chars().next().unwrap_or('\0'),
        _ => return false,
    };
    match c.to_ascii_lowercase() {
        's' => {
            s.menu = None;
            s.save_dialog(event.mods.shift, tx);
        }
        'o' => {
            s.menu = None;
            s.open_dialog(tx);
        }
        _ => return false,
    }
    true
}

pub(crate) fn controls(s: &State) -> El {
    text("")
        .key("file-status")
        .pin(None, None, Some(d::S4), Some(d::S4))
        .live_text(&s.files.status.0)
        .size(d::FILE_STATUS_WIDTH, d::S4)
        .align(rune::Align::End)
        .font(d::T_HINT)
        .color(d::themed(d::MUTED, s.editor.settings().dark))
}

pub(crate) fn menu(s: &State) -> El {
    let dark = s.editor.settings().dark;
    col()
        .w_full()
        .child(
            menu_action("New tab", "new-canvas-tab", "Ctrl+T", dark)
                .on_press_tx(|s: &mut State, tx| s.new_tab(tx)),
        )
        .gap(d::S1)
        .child(
            menu_action("Open", "open-file", "Ctrl+O", dark).on_press_tx(|s: &mut State, tx| {
                s.menu = None;
                s.open_dialog(tx);
            }),
        )
        .child(
            menu_action("Save", "save-file", "Ctrl+S", dark)
                .help("Save · Ctrl+S · Save as · Ctrl+Shift+S")
                .on_press_tx(|s: &mut State, tx| {
                    s.menu = None;
                    s.save_dialog(false, tx);
                }),
        )
        .child(
            menu_action("Export…", "export-file", "Ctrl+Shift+E", dark).on_press(
                |s: &mut State| {
                    s.menu = None;
                    s.export.open = true;
                },
            ),
        )
        .child(el().w_full().h(d::OUTLINE).fill(d::themed(d::BORDER, dark)))
        .child(
            menu_action("Reset canvas", "reset-canvas", "", dark).on_press(|s: &mut State| {
                s.menu = None;
                if !s.editor.modes().view {
                    s.files.confirm_reset = true;
                }
            }),
        )
}

fn menu_action(label: &str, key: &str, shortcut: &str, dark: bool) -> El {
    row()
        .key(key)
        .w_full()
        .h(d::CONTROL)
        .px(d::S2)
        .items_center()
        .justify_between()
        .role("menuitem")
        .label(label)
        .radius(d::R_CONTROL)
        .hover(d::themed(d::ACCENT_SOFT, dark))
        .child(
            text(label)
                .font(d::T_CONTROL)
                .color(d::themed(d::INK, dark)),
        )
        .child(
            text(shortcut)
                .font(d::T_CAPTION)
                .color(d::themed(d::MUTED, dark)),
        )
}

pub(crate) fn button(label: &str, key: &str, dark: bool) -> El {
    el().key(key)
        .h(d::BUTTON)
        .padding(d::S3)
        .items_center()
        .justify_center()
        .child(text(label).font(d::T_LABEL).color(d::themed(d::INK, dark)))
        .radius(d::R_BUTTON)
        .hover(d::themed(d::ACCENT_SOFT, dark))
}

pub(crate) fn confirmation(s: &State) -> El {
    let dark = s.editor.settings().dark;
    if !s.files.confirm_reset {
        return el().size(0.0, 0.0);
    }
    el().key("reset-overlay").absolute(0.0, 0.0).w_full().h_full().items_center().justify_center()
        .fill(d::themed(d::SELECTION_WASH,dark))
        .child(d::surface(col(), d::R_ISLAND,dark).w(d::CONFIRM_WIDTH).padding(d::S4).gap(d::S4)
            .modal(false, |s: &mut State| s.files.confirm_reset = false)
            .child(text("Reset canvas?").font(d::T_LABEL).color(d::themed(d::INK,dark)))
            .child(text("This clears the canvas and its local recovery. Save a copy first to keep it.").wrap().font(d::T_HINT).color(d::themed(d::MUTED,dark)))
            .child(row().gap(d::S3)
                .child(button("Cancel", "cancel-reset",dark).on_press(|s: &mut State| s.files.confirm_reset = false))
                .child(button("Reset canvas", "confirm-reset",dark).fill(d::themed(d::ACCENT,dark)).on_press_tx(|s: &mut State, tx| {
                    if s.editor.modes().view { s.files.confirm_reset=false; return; }
                    let camera = Camera::centred_on(WorldPt::ORIGIN, s.editor.viewport(), 1.0);
                    let scene = SceneFile::new(&Document::new(), AppState { scroll: camera.scroll(), ..AppState::default() });
                    s.install(&scene);
                    s.files.path = None; s.files.mark_dirty(); s.files.refresh_label(); s.files.confirm_reset = false; s.files.changed(tx); s.autosave();
                }))))
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
