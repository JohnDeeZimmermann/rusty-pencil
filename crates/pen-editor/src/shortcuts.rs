use crate::{Editor, Key, Modifiers, Redraw, Selection, Tool};
use pen_core::Command;

impl Editor {
    pub(crate) fn key(&mut self, key: Key, mods: Modifiers) -> Redraw {
        if mods.ctrl && mods.shift && matches!(key, Key::Char('<' | '>' | ',' | '.')) {
            let (_, size, _) = self.text_style();
            let sizes = [16.0, 20.0, 28.0, 36.0];
            let larger = matches!(key, Key::Char('>' | '.'));
            let size = if larger {
                sizes.into_iter().find(|s| *s > size).unwrap_or(36.0)
            } else {
                sizes.into_iter().rev().find(|s| *s < size).unwrap_or(16.0)
            };
            let (font, _, align) = self.text_style();
            return self.set_text_style(font, size, align);
        }
        if let Key::Char(c) = key {
            let settings = &mut self.settings;
            if c == 'd' && mods.alt && mods.shift && !mods.ctrl {
                settings.dark = !settings.dark;
                return Redraw::Scene;
            }
            if c == '\'' && mods.ctrl && !mods.alt {
                settings.grid = !settings.grid;
                return Redraw::Scene;
            }
            if c == 'q' && !mods.ctrl && !mods.alt {
                settings.tool_lock = !settings.tool_lock;
                return Redraw::Tool;
            }
        }
        if self.text_session.is_some() {
            return Redraw::Nothing;
        }
        if let Some(redraw) = self.view_key(key, mods) {
            return redraw;
        }
        if key == Key::Enter {
            if mods.ctrl {
                return self.begin_point_edit();
            }
            if self.point_editor.take().is_some() {
                return Redraw::Tool;
            }
            return self.finish_path(true);
        }
        if matches!(key, Key::Backspace | Key::Delete) {
            if let Some(edit) = &mut self.point_editor {
                if edit.cancel() {
                    return Redraw::Draft;
                }
                let command = edit.delete(&self.document);
                return if command.is_some_and(|c| self.execute(c, self.selection.clone())) {
                    Redraw::Scene
                } else {
                    Redraw::Nothing
                };
            }
            if let Some(crate::Gesture::Path(draw)) = &mut self.gesture {
                draw.backspace();
                return Redraw::Draft;
            }
        }
        if let Some(action) = crate::Action::for_key(key, mods) {
            return self.action(action);
        }
        if mods.ctrl && !mods.alt {
            if let Key::Char(c) = key {
                match c.to_ascii_lowercase() {
                    'a' => {
                        self.cancel();
                        self.tool = Tool::Select;
                        self.point_editor = None;
                        self.selection = Selection::all(&self.document);
                        return Redraw::Tool;
                    }
                    'z' | 'y' => {
                        if self.cancel() {
                            return Redraw::Draft;
                        }
                        let redo = c.eq_ignore_ascii_case(&'y') || mods.shift;
                        return match self.history.step(
                            &mut self.document,
                            &mut self.selection,
                            redo,
                        ) {
                            Ok(true) => {
                                self.point_editor = None;
                                self.navigation.refresh(&self.document);
                                if !self.selection.is_empty() {
                                    self.tool = Tool::Select;
                                }
                                Redraw::Scene
                            }
                            _ => Redraw::Nothing,
                        };
                    }
                    _ => {}
                }
            }
            return Redraw::Nothing;
        }
        if let Some(tool) = Tool::for_key(key, mods) {
            return self.select_tool(tool);
        }
        if self.gesture.is_some() || mods.ctrl || mods.alt {
            return Redraw::Nothing;
        }
        if self.selection.elements(&self.document).any(|e| e.locked) {
            return Redraw::Nothing;
        }
        let before = self.selection.clone();
        if before.is_empty() {
            return Redraw::Nothing;
        }
        let commands = if matches!(key, Key::Delete | Key::Backspace) {
            self.selection.clear();
            before
                .ids()
                .map(|id| Command::Remove { id })
                .collect::<Vec<_>>()
        } else {
            let step = if self.settings.grid {
                crate::GRID_SPACING * if mods.shift { 5.0 } else { 1.0 }
            } else {
                (if mods.shift { 10.0 } else { 1.0 }) / self.camera.zoom()
            };
            let (dx, dy) = match key {
                Key::Left => (-step, 0.0),
                Key::Right => (step, 0.0),
                Key::Up => (0.0, -step),
                Key::Down => (0.0, step),
                _ => return Redraw::Nothing,
            };
            self.selection
                .elements(&self.document)
                .cloned()
                .map(|before| {
                    let after = before.with_bounds(before.kind.bounds().translated(dx, dy));
                    Command::Patch {
                        id: before.id(),
                        before,
                        after,
                    }
                })
                .collect()
        };
        if self.execute(Command::Group(commands), before.clone()) {
            Redraw::Scene
        } else {
            self.selection = before;
            Redraw::Nothing
        }
    }
}
