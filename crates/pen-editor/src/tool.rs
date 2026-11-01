use crate::{Key, Modifiers};

/// The active tool. More arrive with later milestones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tool {
    Select,
    #[default]
    Rectangle,
    Diamond,
    Ellipse,
    Arrow,
    Line,
    Freedraw,
    Text,
    Hand,
}

impl Tool {
    /// Every tool, in toolbar order.
    pub const ALL: [Tool; 9] = [
        Tool::Select,
        Tool::Rectangle,
        Tool::Diamond,
        Tool::Ellipse,
        Tool::Arrow,
        Tool::Line,
        Tool::Freedraw,
        Tool::Text,
        Tool::Hand,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Selection",
            Tool::Rectangle => "Rectangle",
            Tool::Diamond => "Diamond",
            Tool::Ellipse => "Ellipse",
            Tool::Arrow => "Arrow",
            Tool::Line => "Line",
            Tool::Freedraw => "Pen",
            Tool::Text => "Text",
            Tool::Hand => "Hand",
        }
    }

    /// The letter shortcut, as Excalidraw binds it.
    #[must_use]
    pub fn letter(self) -> char {
        match self {
            Tool::Select => 'v',
            Tool::Rectangle => 'r',
            Tool::Diamond => 'd',
            Tool::Ellipse => 'o',
            Tool::Arrow => 'a',
            Tool::Line => 'l',
            Tool::Freedraw => 'p',
            Tool::Text => 't',
            Tool::Hand => 'h',
        }
    }

    /// The number shortcut, matching the toolbar position in Excalidraw.
    #[must_use]
    pub fn digit(self) -> char {
        match self {
            Tool::Select => '1',
            Tool::Rectangle => '2',
            Tool::Diamond => '3',
            Tool::Ellipse => '4',
            Tool::Arrow => '5',
            Tool::Line => '6',
            Tool::Freedraw => '7',
            Tool::Text => '8',
            Tool::Hand => 'h',
        }
    }

    /// The tool a plain key press selects, if any. Chords with Ctrl/Cmd or Alt are left for
    /// commands.
    #[must_use]
    pub fn for_key(key: Key, mods: Modifiers) -> Option<Tool> {
        let Key::Char(c) = key else { return None };
        if mods.ctrl || mods.alt {
            return None;
        }
        let c = c.to_ascii_lowercase();
        Tool::ALL
            .into_iter()
            .find(|t| t.letter() == c || t.digit() == c)
    }
}
