use crate::{Editor, Redraw, Selection, Tool};
use pen_core::{Command, ElementId, Property, Props, Settings, Value, WorldPt};

pub const GRID_SPACING: f32 = 20.0;

#[derive(Debug, Clone)]
pub(crate) struct PropertyEdit {
    pub(crate) defaults: Props,
    pub(crate) elements: Vec<pen_core::Element>,
    text: Option<Props>,
}

pub(crate) fn snap(p: WorldPt, grid: bool) -> WorldPt {
    if grid {
        WorldPt::new(
            (p.x / GRID_SPACING).round() * GRID_SPACING,
            (p.y / GRID_SPACING).round() * GRID_SPACING,
        )
    } else {
        p
    }
}

impl Editor {
    #[must_use]
    pub fn settings(&self) -> Settings {
        self.settings
    }

    /// Defaults when unselected, otherwise a value shared by every selected element.
    #[must_use]
    pub fn property<T: Copy + PartialEq>(&self, field: impl Fn(&Props) -> T) -> Value<T> {
        if let Some(session) = &self.text_session {
            return Value::Uniform(field(&session.element.props));
        }
        if let Some(edit) = &self.property_edit
            && let Some(first) = edit.elements.first()
        {
            let value = field(&first.props);
            return if edit.elements.iter().all(|e| field(&e.props) == value) {
                Value::Uniform(value)
            } else {
                Value::Mixed
            };
        }
        let mut elements = self.selection.elements(&self.document);
        let Some(first) = elements.next() else {
            return Value::Uniform(field(&self.style));
        };
        let value = field(&first.props);
        if elements.all(|e| field(&e.props) == value) {
            Value::Uniform(value)
        } else {
            Value::Mixed
        }
    }

    pub(crate) fn set_property(&mut self, property: Property) -> Redraw {
        if self.selection.elements(&self.document).any(|e| e.locked) {
            return Redraw::Nothing;
        }
        property.apply(&mut self.style);
        if let Some(session) = &mut self.text_session {
            property.apply(&mut session.element.props);
            return Redraw::Draft;
        }
        if let Some(edit) = &mut self.property_edit {
            for element in &mut edit.elements {
                property.apply(&mut element.props);
            }
            return Redraw::Draft;
        }
        let targets: Vec<_> = self
            .selection
            .elements(&self.document)
            .chain(self.styled_texts().filter(|e| {
                !self.selection.contains(e.id()) && matches!(property, Property::VerticalAlign(_))
            }))
            .collect();
        let commands = targets
            .into_iter()
            .filter_map(|before| {
                let mut after = before.clone();
                property.apply(&mut after.props);
                (after.props != before.props).then(|| Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                })
            })
            .collect::<Vec<_>>();
        if !commands.is_empty() {
            self.execute(Command::Group(commands), self.selection.clone());
        }
        Redraw::Scene
    }

    pub(crate) fn begin_property(&mut self) -> Redraw {
        if self.selection.elements(&self.document).any(|e| e.locked) {
            return Redraw::Nothing;
        }
        self.cancel();
        self.property_edit = Some(PropertyEdit {
            defaults: self.style,
            elements: self.selection.elements(&self.document).cloned().collect(),
            text: self
                .text_session
                .as_ref()
                .map(|session| session.element.props),
        });
        Redraw::Draft
    }

    pub(crate) fn end_property(&mut self, cancel: bool) -> Redraw {
        let Some(edit) = self.property_edit.take() else {
            return Redraw::Nothing;
        };
        if cancel {
            self.style = edit.defaults;
            if let (Some(before), Some(session)) = (edit.text, &mut self.text_session) {
                session.element.props = before;
            }
            return Redraw::Draft;
        }
        if edit.text.is_some() {
            return Redraw::Draft;
        }
        let commands = edit
            .elements
            .into_iter()
            .filter_map(|after| {
                let before = self.document.get(after.id())?;
                (before.props != after.props).then(|| Command::Patch {
                    id: after.id(),
                    before: before.clone(),
                    after,
                })
            })
            .collect::<Vec<_>>();
        if !commands.is_empty() {
            self.execute(Command::Group(commands), self.selection.clone());
        }
        Redraw::Scene
    }

    pub(crate) fn commit_drawing(&mut self, command: Command, id: ElementId) -> Redraw {
        let before = self.selection.clone();
        if !self.settings.tool_lock {
            self.selection = Selection::only(id);
        }
        if self.execute(command, before.clone()) {
            if !self.settings.tool_lock {
                self.tool = Tool::Select;
            }
            Redraw::Scene
        } else {
            self.selection = before;
            Redraw::Draft
        }
    }
}

impl PropertyEdit {
    pub(crate) fn text_is_absent(&self) -> bool {
        self.text.is_none()
    }
}
