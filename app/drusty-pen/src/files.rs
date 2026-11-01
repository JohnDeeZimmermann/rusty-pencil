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

#[derive(Debug, Default)]
pub(crate) struct Files {
    pub(crate) path: Option<PathBuf>,
    pub(crate) recovery: Option<PathBuf>,
    pending: bool,
    deadline: Option<f64>,
    pub(crate) status: Status,
    confirm_reset: bool,
}
#[derive(Debug)]
pub(crate) struct Status(rune::LiveText);
impl Default for Status {
    fn default() -> Self {
        Self(rune::LiveText::new("Local canvas"))
    }
}
impl Status {
    fn set(&self, value: impl Into<std::sync::Arc<str>>) {
        self.0.set(value);
    }
}

impl Files {
    pub(crate) fn changed(&mut self, tx: &mut Tx) {
        self.pending = true;
        self.deadline = None;
        self.status.set("Unsaved changes");
        tx.wake_after(Duration::from_secs(1));
    }
}

impl State {
    fn snapshot(&self) -> SceneFile {
        let mut scene = SceneFile::new(
            self.editor.document(),
            AppState {
                settings: self.editor.settings(),
                scroll: self.editor.camera().scroll(),
                zoom: self.editor.camera().zoom(),
                style: self.editor.style(),
                font: self.editor.text_defaults().0,
                font_size: self.editor.text_defaults().1,
                text_align: self.editor.text_defaults().2,
            },
        );
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
        if let Some(path) = self.files.recovery.clone().filter(|p| p.exists()) {
            match read(&path) {
                Ok(scene) => {
                    self.install(&scene);
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
            match read(Path::new(path)) {
                Ok(scene) => {
                    self.install(&scene);
                    self.files.path = Some(path.into());
                }
                Err(error) => self.files.status.set(format!("Could not open: {error}")),
            }
        }
    }

    fn install(&mut self, scene_file: &SceneFile) {
        let Ok(doc) = scene_file.document() else {
            return;
        };
        let app = &scene_file.app_state;
        self.editor
            .replace_document(doc, Camera::new(app.scroll, app.zoom), app.style);
        self.editor.handle(InputEvent::Settings(app.settings));
        self.editor
            .restore_text_defaults(app.font, app.font_size, app.text_align);
        self.scene = scene::Scene::default();
        self.text_edit = rune::TextEdit::default();
        self.camera.show(self.editor.camera());
        self.grid.show(self.editor.camera());
        crate::properties::sync(self);
        self.overlay.show(&self.editor);
        self.show_feedback();
        self.content_button.set(if self.editor.content_offscreen() {
            1.0
        } else {
            0.0
        });
    }

    pub(crate) fn open_path(&mut self, path: &Path, tx: &mut Tx) {
        match read(path) {
            Ok(scene) => {
                self.install(&scene);
                self.files.path = Some(path.to_owned());
                self.files.changed(tx);
                self.files.status.set(format!("Opened {}", name(path)));
            }
            Err(error) => self.files.status.set(format!("Could not open: {error}")),
        }
    }

    fn save_path(&mut self, path: &Path) {
        match write(path, &self.snapshot()) {
            Ok(()) => {
                self.files.path = Some(path.to_owned());
                self.files.status.set(format!("Saved {}", name(path)));
            }
            Err(error) => self.files.status.set(format!("Could not save: {error}")),
        }
    }

    fn save_dialog(&mut self, save_as: bool, tx: &mut Tx) {
        self.sync_text(tx);
        self.input(InputEvent::FinishText, tx);
        if !save_as && let Some(path) = self.files.path.clone() {
            self.save_path(&path);
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
        .filter("drusty-pen scene", &["drusty-pen", "json"]);
        tx.save_file(options, |s: &mut State, result, _| match result {
            Ok(Some(path)) => s.save_path(&path),
            Ok(None) => {}
            Err(e) => s.files.status.set(format!("Could not save: {e}")),
        });
    }

    fn open_dialog(&mut self, tx: &mut Tx) {
        self.sync_text(tx);
        self.input(InputEvent::FinishText, tx);
        tx.open_file(
            FileDialogOptions {
                title: "Open canvas".into(),
                ..Default::default()
            }
            .filter("drusty-pen scene", &["drusty-pen", "json"]),
            |s: &mut State, result, tx| match result {
                Ok(Some(path)) => s.open_path(&path, tx),
                Ok(None) => {}
                Err(e) => s.files.status.set(format!("Could not open: {e}")),
            },
        );
    }

    fn autosave(&mut self) -> bool {
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

fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn read(path: &Path) -> Result<SceneFile, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    file::load(&bytes).map_err(|e| e.to_string())
}

fn write(path: &Path, scene: &SceneFile) -> Result<(), String> {
    let bytes = file::save(scene).map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
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
        's' => s.save_dialog(event.mods.shift, tx),
        'o' => s.open_dialog(tx),
        _ => return false,
    }
    true
}

pub(crate) fn controls(s: &State) -> El {
    let dark = s.editor.settings().dark;
    col()
        .key("file-controls")
        .pin(None, None, Some(d::S4), Some(d::S4))
        .gap(d::S3)
        .items_end()
        .child(
            text("")
                .live_text(&s.files.status.0)
                .size(d::FILE_STATUS_WIDTH, d::S4)
                .align(rune::Align::End)
                .font(d::T_HINT)
                .color(d::themed(d::MUTED, dark)),
        )
        .child(
            d::surface(row(), d::R_ISLAND, dark)
                .padding(d::PADDING)
                .gap(d::S1)
                .child(
                    button("Open", "open-file", dark)
                        .help("Open canvas · Ctrl+O")
                        .on_press_tx(|s: &mut State, tx| s.open_dialog(tx)),
                )
                .child(
                    button("Save", "save-file", dark)
                        .help("Save · Ctrl+S · Save as · Ctrl+Shift+S")
                        .on_press_tx(|s: &mut State, tx| s.save_dialog(false, tx)),
                )
                .child(
                    button("Reset canvas", "reset-canvas", dark)
                        .on_press(|s: &mut State| s.files.confirm_reset = true),
                ),
        )
}

fn button(label: &str, key: &str, dark: bool) -> El {
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
                    let camera = Camera::centred_on(WorldPt::ORIGIN, s.editor.viewport(), 1.0);
                    let scene = SceneFile::new(&Document::new(), AppState { scroll: camera.scroll(), ..AppState::default() });
                    s.install(&scene);
                    s.files.path = None; s.files.confirm_reset = false; s.files.changed(tx); s.autosave();
                }))))
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
