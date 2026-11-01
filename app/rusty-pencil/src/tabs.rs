//! Each inactive tab owns its editor and retained paint; switching never clones a document.
use crate::{State, design as d, files::Files, scene::Scene};
use pen_core::WorldPt;
use pen_editor::{Camera, Editor, InputEvent};
use rune::{Cursor, El, Key, KeyEvent, Tx, col, el, row, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct TabId(pub(crate) u64);

#[derive(Debug)]
pub(crate) struct Canvas {
    pub(crate) editor: Editor,
    pub(crate) files: Files,
    pub(crate) scene: Scene,
}

#[derive(Debug)]
pub(crate) struct Tab {
    pub(crate) id: TabId,
    /// Exactly the active entry has no parked canvas: State owns it while it is visible.
    pub(crate) parked: Option<Canvas>,
}

#[derive(Debug)]
pub(crate) struct Tabs {
    pub(crate) entries: Vec<Tab>,
    pub(crate) active: TabId,
    pub(crate) next: u64,
    pub(crate) closing: Option<TabId>,
    pub(crate) workspace: Option<std::path::PathBuf>,
}
impl Default for Tabs {
    fn default() -> Self {
        Self {
            entries: vec![Tab {
                id: TabId(1),
                parked: None,
            }],
            active: TabId(1),
            next: 2,
            closing: None,
            workspace: None,
        }
    }
}

impl State {
    pub(crate) fn finish_tab_input(&mut self, tx: &mut Tx) {
        let mut finishing = Tx::default();
        self.sync_text(&mut finishing);
        self.input(InputEvent::FinishText, &mut finishing);
        self.editor.handle(InputEvent::Cancel);
        self.editor.handle(InputEvent::PanEnd);
        self.editor.handle(InputEvent::Space(false));
        self.refresh_canvas();
        self.context = None;
        self.menu = None;
        self.export.open = false;
        tx.blur();
        tx.instant();
    }

    pub(crate) fn switch_tab(&mut self, id: TabId, tx: &mut Tx) {
        if id == self.tabs.active || self.tabs.closing.is_some() {
            return;
        }
        let Some(at) = self
            .tabs
            .entries
            .iter()
            .position(|t| t.id == id && t.parked.is_some())
        else {
            return;
        };
        self.finish_tab_input(tx);
        if self.files.recovery.as_ref().is_some_and(|p| !p.exists()) {
            self.files.changed(tx);
        }
        self.autosave();
        let Some(mut canvas) = self.tabs.entries[at].parked.take() else {
            return;
        };
        let viewport = self.editor.viewport();
        std::mem::swap(&mut self.editor, &mut canvas.editor);
        std::mem::swap(&mut self.files, &mut canvas.files);
        std::mem::swap(&mut self.scene, &mut canvas.scene);
        if let Some(old) = self
            .tabs
            .entries
            .iter_mut()
            .find(|t| t.id == self.tabs.active)
        {
            old.parked = Some(canvas);
        }
        self.tabs.active = id;
        self.editor.handle(InputEvent::Viewport(viewport));
        self.refresh_canvas();
        self.persist_workspace();
        tx.scroll_into_view(format!("tab-{}", id.0));
    }

    pub(crate) fn new_tab(&mut self, tx: &mut Tx) {
        if self.tabs.closing.is_some() {
            return;
        }
        let Some(next) = self.tabs.next.checked_add(1) else {
            self.files.status.set("Could not create another tab");
            return;
        };
        let id = TabId(self.tabs.next);
        self.tabs.next = next;
        let camera = Camera::centred_on(WorldPt::ORIGIN, self.editor.viewport(), 1.0);
        let mut files = Files::default();
        files.untitled = format!("Untitled {}", id.0);
        files.recovery = self
            .tabs
            .workspace
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.join(format!("tab-{}.drusty-pen", id.0)));
        files.refresh_label();
        self.tabs.entries.push(Tab {
            id,
            parked: Some(Canvas {
                editor: Editor::new(camera),
                files,
                scene: Scene::default(),
            }),
        });
        self.switch_tab(id, tx);
        // An empty tab also needs a recovery file before its manifest can reference it.
        self.files.changed(tx);
        self.autosave();
    }

    pub(crate) fn refresh_canvas(&mut self) {
        self.sync_appearance();
        self.text_edit = rune::TextEdit::default();
        self.camera.show(self.editor.camera());
        self.grid.show(self.editor.camera());
        crate::properties::sync(self);
        self.polish.modal = None;
        self.polish.frame.focused = false;
        for field in &mut self.polish.geometry {
            field.focused = false;
        }
        self.library.dragging = None;
        crate::details::sync(self);
        self.overlay.show(&self.editor);
        self.show_feedback();
        self.content_button.set(if self.editor.content_offscreen() {
            1.0
        } else {
            0.0
        });
    }

    pub(crate) fn request_close_tab(&mut self, id: TabId, tx: &mut Tx) {
        if self.tabs.closing.is_some() {
            return;
        }
        self.switch_tab(id, tx);
        self.finish_tab_input(tx);
        if self.files.dirty {
            self.tabs.closing = Some(id);
        } else {
            self.close_tab(id, tx);
        }
    }

    pub(crate) fn close_tab(&mut self, id: TabId, tx: &mut Tx) {
        self.tabs.closing = None;
        let Some(at) = self.tabs.entries.iter().position(|t| t.id == id) else {
            return;
        };
        if self.tabs.entries.len() == 1 {
            self.new_tab(tx);
        }
        if self.tabs.active == id {
            let next = if at + 1 < self.tabs.entries.len() {
                at + 1
            } else {
                at - 1
            };
            self.switch_tab(self.tabs.entries[next].id, tx);
        }
        let removed = self.tabs.entries.remove(at);
        if self.persist_workspace()
            && let Some(path) = removed.parked.and_then(|c| c.files.recovery)
            && let Some(workspace) = &self.tabs.workspace
            && path.parent() == workspace.parent()
            && path.file_name().is_some_and(|n| {
                n == "autosave.drusty-pen" || n == format!("tab-{}.drusty-pen", id.0).as_str()
            })
            && let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            self.files
                .status
                .set(format!("Could not remove closed tab recovery: {error}"));
        }
    }
}

pub(crate) fn key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if s.tabs.closing.is_some() {
        if ev.key == Key::Escape {
            s.tabs.closing = None;
            tx.blur();
        }
        return true;
    }
    let command = match &ev.key {
        Key::Command(c) => Some(c.to_ascii_lowercase()),
        Key::Text(t) if t.len() == 1 => t.chars().next().map(|c| c.to_ascii_lowercase()),
        _ => None,
    };
    let cycle =
        (ev.mods.ctrl || ev.mods.cmd) && matches!(ev.key, Key::Tab | Key::PageUp | Key::PageDown);
    if cycle {
        let len = s.tabs.entries.len();
        let at = s
            .tabs
            .entries
            .iter()
            .position(|t| t.id == s.tabs.active)
            .unwrap_or(0);
        let back = ev.mods.shift || ev.key == Key::PageUp;
        let next = if back {
            (at + len - 1) % len
        } else {
            (at + 1) % len
        };
        s.switch_tab(s.tabs.entries[next].id, tx);
        return true;
    }
    if !ev.mods.cmd || ev.mods.alt {
        return false;
    }
    match command {
        Some('t' | 'n') if !ev.mods.shift => s.new_tab(tx),
        Some('w') if !ev.mods.shift => s.request_close_tab(s.tabs.active, tx),
        Some(c @ '1'..='9') => {
            let at = if c == '9' {
                s.tabs.entries.len() - 1
            } else {
                (c as u8 - b'1') as usize
            };
            if let Some(t) = s.tabs.entries.get(at) {
                s.switch_tab(t.id, tx);
            }
        }
        _ => return false,
    }
    true
}

pub(crate) fn view(s: &State) -> El {
    let dark = s.editor.settings().dark;
    let mut list = row().gap(d::S1).items_center();
    for tab in &s.tabs.entries {
        let id = tab.id;
        let files = tab.parked.as_ref().map_or(&s.files, |c| &c.files);
        let active = id == s.tabs.active;
        list = list.child(
            row()
                .key(format!("tab-{}", id.0))
                .size(d::TAB_WIDTH, d::CONTROL)
                .radius(d::R_BUTTON)
                .fill(d::themed(if active { d::ACCENT } else { d::PAPER }, dark))
                .child(
                    el().key(format!("select-tab-{}", id.0))
                        .grow(1.0)
                        .h_full()
                        .px(d::S3)
                        .items_center()
                        .justify_center()
                        .role("tab")
                        .focus_selected(active)
                        .label(files.title())
                        .help(
                            files
                                .path
                                .as_ref()
                                .map_or_else(|| files.title(), |p| p.display().to_string()),
                        )
                        .hover(d::themed(d::ACCENT_SOFT, dark))
                        .child(
                            text("")
                                .live_text(&files.label)
                                .font(d::T_LABEL)
                                .color(d::themed(
                                    if active { d::ON_ACCENT } else { d::MUTED },
                                    dark,
                                ))
                                .size(d::TAB_WIDTH - d::CONTROL - d::S3 * 2.0, d::S4)
                                .truncate_fade(),
                        )
                        .on_press_tx(move |s: &mut State, tx| s.switch_tab(id, tx)),
                )
                .child(
                    el().key(format!("close-tab-{}", id.0))
                        .size(d::CONTROL, d::CONTROL)
                        .radius(d::R_BUTTON)
                        .role("button")
                        .label("Close tab")
                        .help("Close tab · Ctrl+W")
                        .items_center()
                        .justify_center()
                        .hover(d::themed(d::ACCENT_SOFT, dark))
                        .child(text("×").font(d::T_GLYPH).color(d::themed(d::MUTED, dark)))
                        .on_press_tx(move |s: &mut State, tx| s.request_close_tab(id, tx)),
                ),
        );
    }
    row()
        .key("document-tabs")
        .absolute(0.0, 0.0)
        .w_full()
        .h(d::TAB_BAR_HEIGHT)
        .px(d::S4)
        .gap(d::S2)
        .items_center()
        .fill(d::themed(d::PAPER, dark))
        .border(d::OUTLINE, d::themed(d::BORDER, dark))
        .cursor(Cursor::Default)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only())
        .child(
            el().key("tab-scroll")
                .grow(1.0)
                .h(d::CONTROL)
                .scroll_x()
                .child(list),
        )
        .child(
            el().key("new-tab")
                .size(d::CONTROL, d::CONTROL)
                .radius(d::R_BUTTON)
                .role("button")
                .label("New tab")
                .help("New canvas · Ctrl+T / Ctrl+N")
                .items_center()
                .justify_center()
                .hover(d::themed(d::ACCENT_SOFT, dark))
                .child(text("+").font(d::T_GLYPH).color(d::themed(d::MUTED, dark)))
                .on_press_tx(|s: &mut State, tx| s.new_tab(tx)),
        )
}

pub(crate) fn confirmation(s: &State) -> El {
    if s.tabs.closing.is_none() {
        return el().size(0.0, 0.0);
    }
    let dark = s.editor.settings().dark;
    el().key("close-tab-overlay")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .fill(d::themed(d::SELECTION_WASH, dark))
        .child(
            d::surface(col(), d::R_ISLAND, dark)
                .w(d::CONFIRM_WIDTH)
                .padding(d::S4)
                .gap(d::S4)
                .modal(false, |s: &mut State| s.tabs.closing = None)
                .child(
                    text(format!("Save {}?", s.files.title()))
                        .font(d::T_LABEL)
                        .color(d::themed(d::INK, dark)),
                )
                .child(
                    text("Save your changes to a file before closing this tab.")
                        .wrap()
                        .font(d::T_HINT)
                        .color(d::themed(d::MUTED, dark)),
                )
                .child(
                    row()
                        .gap(d::S2)
                        .child(
                            crate::files::button("Cancel", "cancel-close-tab", dark)
                                .on_press(|s: &mut State| s.tabs.closing = None),
                        )
                        .child(
                            crate::files::button("Discard", "discard-close-tab", dark).on_press_tx(
                                |s: &mut State, tx| {
                                    if let Some(id) = s.tabs.closing {
                                        s.close_tab(id, tx);
                                    }
                                },
                            ),
                        )
                        .child(
                            crate::files::button("Save", "save-close-tab", dark)
                                .fill(d::themed(d::ACCENT, dark))
                                .on_press_tx(|s: &mut State, tx| s.save_dialog(false, tx)),
                        ),
                ),
        )
}

#[cfg(test)]
#[path = "tab_tests.rs"]
mod tests;
