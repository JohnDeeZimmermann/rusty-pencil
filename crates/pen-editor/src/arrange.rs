//! Atomic arrange operations. Only affected element snapshots enter history.
use crate::{Action, Editor, Redraw, Selection, Tool};
use pen_core::{Command, Element, GroupId, Kind, WorldPt, ZKey};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CopiedStyle {
    pub(crate) props: pen_core::Props,
    pub(crate) text: Option<(pen_core::Font, f32, pen_core::TextAlign)>,
}

impl Editor {
    pub(crate) fn action(&mut self, action: Action) -> Redraw {
        if !action.enabled(self) {
            return Redraw::Nothing;
        }
        let before_selection = self.selection.clone();
        let originals: Vec<_> = self.selection.elements(&self.document).cloned().collect();
        match action {
            Action::WrapText => return self.wrap_text(),
            Action::Copy | Action::Cut => {
                self.clipboard = self.copy_elements();
                if action == Action::Copy {
                    return Redraw::Nothing;
                }
            }
            Action::Paste => {
                return self.paste_elements(&self.clipboard.clone(), WorldPt::new(20.0, 20.0));
            }
            Action::Duplicate => {
                return self.paste_elements(&self.copy_elements(), WorldPt::new(20.0, 20.0));
            }
            Action::CopyStyle => return self.copy_style(),
            Action::PasteStyle => {
                return self.paste_style_with(|content, _, size, wrap| {
                    let width = content
                        .lines()
                        .map(|s| s.chars().count())
                        .max()
                        .unwrap_or(1) as f32
                        * size
                        * 0.6;
                    let lines: f32 = content
                        .split('\n')
                        .map(|s| {
                            let width = s.chars().count() as f32 * size * 0.6;
                            wrap.map_or(1.0, |w| (width / w.max(1.0)).ceil().max(1.0))
                        })
                        .sum();
                    (
                        wrap.map_or(width, |w| w.min(width)).max(1.0),
                        lines * size * 1.25,
                    )
                });
            }
            _ => {}
        }
        let mut after = originals.clone();
        let bounds = self.selection.bounds(&self.document).unwrap_or_default();
        match action {
            Action::Delete | Action::Cut => {
                self.selection.clear();
                let commands = originals
                    .iter()
                    .map(|e| Command::Remove { id: e.id() })
                    .collect();
                return self.finish_arrange(commands, before_selection);
            }
            Action::Group => {
                let group = GroupId(
                    self.document
                        .elements()
                        .iter()
                        .flat_map(|e| &e.group_ids)
                        .map(|g| g.0)
                        .max()
                        .unwrap_or(0)
                        .saturating_add(1),
                );
                for e in &mut after {
                    if let Some(at) = self
                        .group_scope
                        .and_then(|scope| e.group_ids.iter().position(|g| *g == scope))
                    {
                        e.group_ids.insert(at, group);
                    } else {
                        e.group_ids.push(group);
                    }
                }
            }
            Action::Ungroup => {
                let groups: Vec<_> = originals
                    .iter()
                    .filter_map(|e| crate::selection::active_group(e, self.group_scope))
                    .collect();
                after = self
                    .document
                    .elements()
                    .iter()
                    .filter(|e| e.group_ids.iter().any(|g| groups.contains(g)))
                    .cloned()
                    .collect();
                for e in &mut after {
                    e.group_ids.retain(|g| !groups.contains(g));
                    if e.kind.container().is_none() {
                        self.selection.insert(e.id());
                    }
                }
            }
            Action::Lock => {
                let lock = after.iter().any(|e| !e.locked);
                for e in &mut after {
                    e.locked = lock;
                }
            }
            Action::UnlockAll => {
                after = self
                    .document
                    .elements()
                    .iter()
                    .filter(|e| e.locked)
                    .cloned()
                    .collect();
                for e in &mut after {
                    e.locked = false;
                }
            }
            Action::FlipHorizontal | Action::FlipVertical => {
                let horizontal = action == Action::FlipHorizontal;
                for e in &mut after {
                    let b = e.kind.bounds();
                    let c = b.centre();
                    let centre = bounds.centre();
                    *e = e.with_bounds(b.translated(
                        if horizontal {
                            2.0 * (centre.x - c.x)
                        } else {
                            0.0
                        },
                        if horizontal {
                            0.0
                        } else {
                            2.0 * (centre.y - c.y)
                        },
                    ));
                    e.angle = -e.angle;
                    if let Some(path) = e.kind.path_mut() {
                        let b = path.bounds();
                        for point in &mut path.points {
                            if horizontal {
                                point.offset.x =
                                    2.0 * (b.centre().x - path.origin.x) - point.offset.x;
                            } else {
                                point.offset.y =
                                    2.0 * (b.centre().y - path.origin.y) - point.offset.y;
                            }
                        }
                    }
                }
            }
            Action::Backward | Action::ToBack | Action::Forward | Action::ToFront => {
                after = self.reordered(action);
            }
            Action::DistributeHorizontal
            | Action::DistributeVertical
            | Action::AlignLeft
            | Action::AlignCentre
            | Action::AlignRight
            | Action::AlignTop
            | Action::AlignMiddle
            | Action::AlignBottom => {
                after = self.arrange_positions(action, &originals);
            }
            _ => return Redraw::Nothing,
        }
        let commands = after
            .into_iter()
            .filter_map(|after| {
                let before = self.document.get(after.id())?;
                (before != &after).then(|| Command::Patch {
                    id: after.id(),
                    before: before.clone(),
                    after,
                })
            })
            .collect();
        self.finish_arrange(commands, before_selection)
    }

    fn finish_arrange(&mut self, commands: Vec<Command>, before: Selection) -> Redraw {
        if !commands.is_empty() && self.execute(Command::Group(commands), before) {
            Redraw::Scene
        } else {
            Redraw::Nothing
        }
    }

    /// Includes bound text; order is the document's paint order.
    #[must_use]
    pub fn copy_elements(&self) -> Vec<Element> {
        self.document
            .elements()
            .iter()
            .filter(|e| {
                self.selection.contains(e.id())
                    || e.kind
                        .container()
                        .is_some_and(|c| self.selection.contains(c.id()))
            })
            .cloned()
            .collect()
    }

    pub(crate) fn paste_elements(&mut self, elements: &[Element], offset: WorldPt) -> Redraw {
        self.paste_with_files(elements, offset, &BTreeMap::new())
    }

    pub(crate) fn paste_with_files(
        &mut self,
        elements: &[Element],
        offset: WorldPt,
        files: &BTreeMap<pen_core::FileId, pen_core::BinaryFile>,
    ) -> Redraw {
        if elements.is_empty() {
            return Redraw::Nothing;
        }
        let before = self.selection.clone();
        let ids: BTreeMap<_, _> = elements
            .iter()
            .map(|e| (e.id(), self.document.alloc_id()))
            .collect();
        let mut next_group = self
            .document
            .elements()
            .iter()
            .flat_map(|e| &e.group_ids)
            .map(|g| g.0)
            .max()
            .unwrap_or(0);
        let mut groups = BTreeMap::new();
        self.selection.clear();
        let at = self.document.elements().len();
        let mut commands: Vec<_> = elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let mut out = e
                    .copied(ids[&e.id()])
                    .with_bounds(e.kind.bounds().translated(offset.x, offset.y));
                out.locked = false;
                for g in &mut out.group_ids {
                    *g = *groups.entry(*g).or_insert_with(|| {
                        next_group += 1;
                        GroupId(next_group)
                    });
                }
                for binding in &mut out.bindings {
                    *binding = binding.and_then(|mut b| {
                        b.element = *ids.get(&b.element)?;
                        Some(b)
                    });
                }
                if let Kind::Text { container, .. } = &mut out.kind {
                    *container = container.and_then(|c| {
                        ids.get(&c.id()).map(|id| match c {
                            pen_core::Container::Shape(_) => pen_core::Container::Shape(*id),
                            pen_core::Container::Path(_) => pen_core::Container::Path(*id),
                        })
                    });
                }
                if out.kind.container().is_none() {
                    self.selection.insert(out.id());
                }
                Command::Add {
                    element: out,
                    at: at + i,
                }
            })
            .collect();
        let file_commands = files
            .iter()
            .filter(|(id, _)| !self.document.files().contains_key(*id))
            .map(|(id, file)| Command::File {
                id: id.clone(),
                file: Some(file.clone()),
            });
        commands.splice(0..0, file_commands);
        self.tool = Tool::Select;
        self.group_scope = None;
        self.finish_arrange(commands, before)
    }

    fn reordered(&self, action: Action) -> Vec<Element> {
        let mut order: Vec<_> = self.document.elements().iter().collect();
        let selected = |e: &&Element| {
            self.selection.contains(e.id())
                || e.kind
                    .container()
                    .is_some_and(|c| self.selection.contains(c.id()))
        };
        match action {
            Action::ToBack => order.sort_by_key(|e| !selected(e)),
            Action::ToFront => order.sort_by_key(selected),
            Action::Backward => {
                for i in 1..order.len() {
                    if selected(&order[i]) && !selected(&order[i - 1]) {
                        order.swap(i, i - 1);
                    }
                }
            }
            _ => {
                for i in (0..order.len().saturating_sub(1)).rev() {
                    if selected(&order[i]) && !selected(&order[i + 1]) {
                        order.swap(i, i + 1);
                    }
                }
            }
        }
        let mut result: Vec<Element> = Vec::new();
        let mut lower = None;
        for (i, e) in order.iter().enumerate() {
            if selected(e) {
                let upper = order[i + 1..]
                    .iter()
                    .find(|e| !selected(e))
                    .map(|e| &e.z_key);
                let mut out = (*e).clone();
                out.z_key = ZKey::between(lower.as_ref(), upper);
                lower = Some(out.z_key.clone());
                result.push(out);
            } else {
                lower = Some(e.z_key.clone());
            }
        }
        result
    }
}
