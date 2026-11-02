//! Window-wide recent scenes, independent of tab recovery and document contents.
use crate::{State, design as d, files, menu::Section, platform};
use pen_core::file::SceneFile;
use rune::{El, Key, KeyEvent, Shape, Tx, canvas, col, el, row, text};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const LIMIT: usize = 10;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Recent {
    paths: Vec<PathBuf>,
    #[serde(skip)]
    pub(crate) selected: Option<usize>,
}

impl State {
    fn recent_directory(&self) -> Option<&Path> {
        self.tabs.workspace.as_deref().and_then(Path::parent)
    }

    pub(crate) fn restore_recent(&mut self) {
        let Some(directory) = self.recent_directory() else {
            return;
        };
        let path = directory.join("recent-files.json");
        if !platform::exists(&path) {
            return;
        }
        match platform::read(&path)
            .and_then(|bytes| serde_json::from_slice::<Recent>(&bytes).map_err(|e| e.to_string()))
        {
            Ok(recent) => {
                self.recent = Recent::default();
                for path in recent.paths.into_iter().take(LIMIT) {
                    if !path.as_os_str().is_empty() {
                        let path = files::identity(&path);
                        if !self.recent.paths.contains(&path) {
                            self.recent.paths.push(path);
                        }
                    }
                }
            }
            Err(error) => self
                .files
                .status
                .set(format!("Could not restore recent files: {error}")),
        }
    }

    fn save_recent(&self) {
        let Some(directory) = self.recent_directory() else {
            return;
        };
        if let Err(error) = serde_json::to_vec(&self.recent)
            .map_err(|e| e.to_string())
            .and_then(|bytes| platform::write(&directory.join("recent-files.json"), &bytes))
        {
            self.files
                .status
                .set(format!("Could not save recent files: {error}"));
        }
    }

    pub(crate) fn remember_recent(&mut self, path: &Path, scene: &SceneFile) {
        let path = files::identity(path);
        if let Some(directory) = self.recent_directory()
            && let Err(error) = platform::cache_recent(directory, &path, scene)
        {
            self.files
                .status
                .set(format!("Could not keep recent file: {error}"));
            return;
        }
        self.recent.paths.retain(|p| p != &path);
        self.recent.paths.insert(0, path);
        if self.recent.paths.len() > LIMIT
            && let Some(removed) = self.recent.paths.pop()
            && let Some(directory) = self.recent_directory()
            && let Err(error) = platform::remove_recent_copy(directory, &removed)
        {
            self.files
                .status
                .set(format!("Could not remove recent copy: {error}"));
        }
        self.recent.selected = None;
        self.save_recent();
    }

    fn clear_recent(&mut self) {
        if let Some(directory) = self.recent_directory() {
            for path in &self.recent.paths {
                if let Err(error) = platform::remove_recent_copy(directory, path) {
                    self.files
                        .status
                        .set(format!("Could not remove recent copy: {error}"));
                }
            }
        }
        self.recent = Recent::default();
        self.save_recent();
    }

    fn open_recent(&mut self, path: &Path, tx: &mut Tx) {
        self.menu = None;
        tx.blur();
        if self.switch_to_open(path, tx) {
            return;
        }
        let source = self.recent_directory().map_or_else(
            || path.to_owned(),
            |directory| platform::recent_source(directory, path),
        );
        match platform::read(&source) {
            Ok(bytes) => self.open_picked(
                &platform::Picked {
                    path: path.to_owned(),
                    bytes,
                },
                tx,
            ),
            Err(error) => self
                .files
                .status
                .set(format!("Could not open {}: {error}", files::name(path))),
        }
    }
}

pub(crate) fn key(s: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if s.menu != Some(Section::Recent) || event.mods.cmd || event.mods.alt {
        return false;
    }
    match event.key {
        Key::Left | Key::Backspace => s.menu = Some(Section::File),
        Key::Up | Key::Down if !s.recent.paths.is_empty() => {
            let len = s.recent.paths.len();
            let at = match (s.recent.selected, &event.key) {
                (None, Key::Up) => len - 1,
                (None, _) => 0,
                (Some(at), Key::Up) => (at + len - 1) % len,
                (Some(at), _) => (at + 1) % len,
            };
            s.recent.selected = Some(at);
            tx.scroll_into_view(format!("recent-file-{at}"));
        }
        Key::Enter => {
            if let Some(path) = s
                .recent
                .selected
                .and_then(|at| s.recent.paths.get(at))
                .cloned()
            {
                s.open_recent(&path, tx);
            }
        }
        _ => return false,
    }
    true
}

pub(crate) fn view(s: &State) -> El {
    let dark = s.editor.settings().dark;
    let mut content = col()
        .w_full()
        .gap(d::S3)
        .child(
            row()
                .key("recent-back")
                .h(d::CONTROL)
                .px(d::S2)
                .radius(d::R_CONTROL)
                .items_center()
                .role("button")
                .label("Back to File menu")
                .hover(d::themed(d::ACCENT_SOFT, dark))
                .child(
                    text("‹  File")
                        .font(d::T_CONTROL)
                        .color(d::themed(d::MUTED, dark)),
                )
                .on_press(|s: &mut State| s.menu = Some(Section::File)),
        )
        .child(
            col()
                .px(d::S2)
                .gap(d::S1)
                .child(
                    row()
                        .items_center()
                        .justify_between()
                        .child(
                            text("Recent files")
                                .font(d::T_LABEL)
                                .color(d::themed(d::INK, dark)),
                        )
                        .child(
                            text(format!("{} / {LIMIT}", s.recent.paths.len()))
                                .font(d::T_CAPTION)
                                .color(d::themed(d::MUTED, dark)),
                        ),
                )
                .child(
                    text("Pick up where you left off.")
                        .font(d::T_HINT)
                        .color(d::themed(d::MUTED, dark)),
                ),
        );
    if s.recent.paths.is_empty() {
        return content.child(
            col()
                .key("recent-empty")
                .w_full()
                .padding(d::S4)
                .gap(d::S3)
                .items_center()
                .child(file_icon(dark))
                .child(
                    text("No recent files yet")
                        .font(d::T_LABEL)
                        .color(d::themed(d::INK, dark)),
                )
                .child(
                    text("Canvases you open or save will appear here.")
                        .wrap()
                        .font(d::T_HINT)
                        .color(d::themed(d::MUTED, dark)),
                )
                .child(
                    files::button("Open a canvas…", "recent-open-file", dark)
                        .fill(d::themed(d::ACCENT, dark))
                        .on_press_tx(|s: &mut State, tx| {
                            s.menu = None;
                            s.open_dialog(tx);
                        }),
                ),
        );
    }
    let mut list = col().w_full().gap(d::S1);
    for (at, path) in s.recent.paths.iter().enumerate() {
        list = list.child(file_row(s, path, at));
    }
    content = content.child(
        list.max_h(
            (s.editor.viewport().dy
                - d::APP_MENU_TOP
                - d::S4 * 2.0
                - d::CONTROL * 3.0
                - d::S3 * 4.0)
                .max(d::RECENT_ROW),
        )
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        }),
    );
    content
        .child(el().w_full().h(d::OUTLINE).fill(d::themed(d::BORDER, dark)))
        .child(
            row()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    text("Newest first")
                        .font(d::T_CAPTION)
                        .color(d::themed(d::MUTED, dark))
                        .px(d::S2),
                )
                .child(
                    el().key("clear-recent")
                        .h(d::CONTROL)
                        .px(d::S2)
                        .radius(d::R_CONTROL)
                        .role("button")
                        .label("Clear recent files")
                        .help("Clear this list; your files and open tabs are kept")
                        .items_center()
                        .justify_center()
                        .hover(d::themed(d::ACCENT_SOFT, dark))
                        .child(
                            text("Clear history")
                                .font(d::T_CONTROL)
                                .color(d::themed(d::MUTED, dark)),
                        )
                        .on_press(|s: &mut State| s.clear_recent()),
                ),
        )
}

fn file_row(s: &State, path: &Path, at: usize) -> El {
    let dark = s.editor.settings().dark;
    let selected = s.recent.selected == Some(at);
    let width = d::RECENT_WIDTH.min(s.editor.viewport().dx - d::S4 * 2.0)
        - d::S3 * 2.0
        - d::S2 * 2.0
        - d::RECENT_ICON
        - d::S3;
    let path = path.to_owned();
    row()
        .key(format!("recent-file-{at}"))
        .w_full()
        .h(d::RECENT_ROW)
        .px(d::S2)
        .gap(d::S3)
        .items_center()
        .radius(d::R_BUTTON)
        .role("menuitem")
        .label(format!("Open {}", files::name(&path)))
        .help(path.display().to_string())
        .fill(d::themed(if selected { d::ACCENT } else { d::PAPER }, dark))
        .hover(d::themed(d::ACCENT_SOFT, dark))
        .child(file_icon(dark))
        .child(
            col()
                .gap(d::S1)
                .child(
                    text(files::name(&path))
                        .size(width, d::S4)
                        .truncate_fade()
                        .font(d::T_LABEL)
                        .color(d::themed(d::INK, dark)),
                )
                .child(
                    text(platform::recent_location(&path))
                        .size(width, d::S4)
                        .truncate_fade()
                        .font(d::T_CAPTION)
                        .color(d::themed(d::MUTED, dark)),
                ),
        )
        .on_press_tx(move |s: &mut State, tx| s.open_recent(&path, tx))
}

fn file_icon(dark: bool) -> El {
    let color = d::themed(d::ON_ACCENT, dark);
    let shape = Shape::new()
        .stroke(
            vec![
                [5.0, 2.0],
                [12.0, 2.0],
                [16.0, 6.0],
                [16.0, 18.0],
                [4.0, 18.0],
                [4.0, 2.0],
                [5.0, 2.0],
            ],
            d::ICON_STROKE,
            color,
        )
        .stroke(
            vec![[11.0, 2.0], [11.0, 7.0], [16.0, 7.0]],
            d::ICON_STROKE,
            color,
        )
        .stroke(vec![[7.0, 11.0], [13.0, 11.0]], d::ICON_STROKE, color)
        .stroke(vec![[7.0, 14.0], [11.0, 14.0]], d::ICON_STROKE, color);
    el().size(d::RECENT_ICON, d::RECENT_ICON)
        .radius(d::R_BUTTON)
        .fill(d::themed(d::ACCENT, dark))
        .items_center()
        .justify_center()
        .child(canvas(shape).size(d::ICON, d::ICON))
}

#[cfg(test)]
#[path = "recent_tests.rs"]
mod tests;
