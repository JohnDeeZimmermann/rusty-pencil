use crate::{Key, Modifiers};

/// The active tool. More arrive with later milestones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tool {
    Select,
    #[default]
    Rectangle,
    Diamond,
    Ellipse,
}

impl Tool {
    /// Every tool, in toolbar order.
    pub const ALL: [Tool; 4] = [Tool::Select, Tool::Rectangle, Tool::Diamond, Tool::Ellipse];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Tool::Select => "Selection",
            Tool::Rectangle => "Rectangle",
            Tool::Diamond => "Diamond",
            Tool::Ellipse => "Ellipse",
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
