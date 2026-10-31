use crate::tools::{
    select::{Action, Phase, SelectionDrag},
    transform::resize,
};
use crate::{
    CLICK_SLOP, Editor, Gesture, Handle, Key, Modifiers, Redraw, ScreenPt, Selection, Tool,
};
use pen_core::{Command, Element, WorldPt};

impl Editor {
    pub(crate) fn selection_down(&mut self, at: ScreenPt, origin: WorldPt, mods: Modifiers) {
        let before = self.selection.clone();
        let handle = if self.selection.len() == 1 {
            self.selection
                .bounds(&self.document)
                .and_then(|r| Handle::hit(r, origin, self.camera.zoom()))
        } else {
            None
        };
        let hit = self
            .document
            .elements()
            .iter()
            .rev()
            .find(|e| e.hit_test(origin, 5.0 / self.camera.zoom()))
            .map(Element::id);
        let action = if let Some(handle) = handle {
            self.selection
                .ids()
                .next()
                .and_then(|id| self.document.get(id))
                .cloned()
                .map(|original| Action::Resize { original, handle })
                .unwrap_or(Action::Box)
        } else if let Some(id) = hit {
            if mods.shift {
                self.selection.toggle(id);
                if !self.selection.contains(id) {
                    Action::Toggle
                } else {
                    Action::Move(self.selected_elements())
                }
            } else {
                if !self.selection.contains(id) {
                    self.selection = Selection::only(id);
                }
                Action::Move(self.selected_elements())
            }
        } else if !mods.shift
            && self
                .selection
                .bounds(&self.document)
                .is_some_and(|r| r.contains(origin))
        {
            // Once selected, a hollow shape can be grabbed by its interior.
            Action::Move(self.selected_elements())
        } else {
            if !mods.shift {
                self.selection.clear();
            }
            Action::Box
        };
        self.gesture = Some(Gesture::Selecting(SelectionDrag {
            action,
            phase: Phase::Pressed,
            before,
            origin,
            current: origin,
            start: at,
            pointer: at,
            mods,
        }));
    }

    fn selected_elements(&self) -> Vec<Element> {
        self.selection
            .ids()
            .filter_map(|id| self.document.get(id))
            .cloned()
            .collect()
    }

    pub(crate) fn selection_update(&mut self, at: Option<ScreenPt>, mods: Modifiers) -> Redraw {
        let Some(Gesture::Selecting(drag)) = &mut self.gesture else {
            return Redraw::Nothing;
        };
        let old = (drag.pointer, drag.current, drag.mods);
        if let Some(at) = at {
            let current = self.camera.screen_to_world(at);
            if current.is_finite() {
                drag.current = current;
                drag.pointer = at;
            }
        }
        drag.mods = mods;
        if old == (drag.pointer, drag.current, drag.mods) {
            return Redraw::Nothing;
        }
        if (drag.pointer.x - drag.start.x).hypot(drag.pointer.y - drag.start.y) >= CLICK_SLOP {
            drag.phase = Phase::Dragging;
        }
        if drag.phase == Phase::Pressed {
            return Redraw::Nothing;
        }
        self.preview.clear();
        let delta = WorldPt::new(
            drag.current.x - drag.origin.x,
            drag.current.y - drag.origin.y,
        );
        match &drag.action {
            Action::Move(originals) => {
                let (dx, dy) = if mods.shift {
                    if delta.x.abs() >= delta.y.abs() {
                        (delta.x, 0.0)
                    } else {
                        (0.0, delta.y)
                    }
                } else {
                    (delta.x, delta.y)
                };
                self.preview.extend(
                    originals
                        .iter()
                        .map(|e| e.with_bounds(e.kind.bounds().translated(dx, dy))),
                );
            }
            Action::Resize { original, handle } => {
                self.preview.push(original.with_bounds(resize(
                    original.kind.bounds(),
                    *handle,
                    delta,
                    mods,
                    self.camera.zoom(),
                )));
            }
            Action::Box | Action::Toggle => {}
        }
        Redraw::Draft
    }

    pub(crate) fn selection_up(&mut self, drag: SelectionDrag) -> Redraw {
        if let Some(bounds) = drag.marquee() {
            // Membership is evaluated once on release: marquee animation doesn't scan the
            // document on every frame. Shift adds to the selection captured at press.
            self.selection = if drag.mods.shift {
                drag.before
            } else {
                Selection::default()
            };
            for element in self.document.elements() {
                if bounds.contains_rect(element.kind.bounds()) {
                    self.selection.insert(element.id());
                }
            }
            return Redraw::Draft;
        }
        let commands = self
            .preview
            .iter()
            .filter_map(|after| {
                let before = self.document.get(after.id())?;
                (before.kind != after.kind).then(|| Command::Patch {
                    id: after.id(),
                    before: before.clone(),
                    after: after.clone(),
                })
            })
            .collect::<Vec<_>>();
        self.preview.clear();
        if !commands.is_empty() && self.execute(Command::Group(commands), drag.before) {
            Redraw::Scene
        } else {
            Redraw::Draft
        }
    }

    pub(crate) fn key(&mut self, key: Key, mods: Modifiers) -> Redraw {
        if mods.ctrl && !mods.alt {
            if let Key::Char(c) = key {
                match c.to_ascii_lowercase() {
                    'a' => {
                        self.cancel();
                        self.tool = Tool::Select;
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
            let step = if mods.shift { 10.0 } else { 1.0 } / self.camera.zoom();
            let (dx, dy) = match key {
                Key::Left => (-step, 0.0),
                Key::Right => (step, 0.0),
                Key::Up => (0.0, -step),
                Key::Down => (0.0, step),
                _ => return Redraw::Nothing,
            };
            self.selected_elements()
                .into_iter()
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
