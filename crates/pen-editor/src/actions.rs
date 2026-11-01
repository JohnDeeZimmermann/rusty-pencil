//! The single registry used by keyboard chords, menus and future command search.
use crate::{Editor, InputEvent, Key, Modifiers, Redraw};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WrapText,
    Duplicate,
    Copy,
    Cut,
    Paste,
    CopyStyle,
    PasteStyle,
    Group,
    Ungroup,
    Lock,
    UnlockAll,
    FlipHorizontal,
    FlipVertical,
    Backward,
    ToBack,
    Forward,
    ToFront,
    AlignLeft,
    AlignCentre,
    AlignRight,
    AlignTop,
    AlignMiddle,
    AlignBottom,
    DistributeHorizontal,
    DistributeVertical,
    Delete,
}

impl Action {
    pub const ALL: [Self; 26] = [
        Self::WrapText,
        Self::Copy,
        Self::Cut,
        Self::Paste,
        Self::Duplicate,
        Self::Delete,
        Self::CopyStyle,
        Self::PasteStyle,
        Self::Group,
        Self::Ungroup,
        Self::Lock,
        Self::UnlockAll,
        Self::FlipHorizontal,
        Self::FlipVertical,
        Self::Backward,
        Self::ToBack,
        Self::Forward,
        Self::ToFront,
        Self::AlignLeft,
        Self::AlignCentre,
        Self::AlignRight,
        Self::AlignTop,
        Self::AlignMiddle,
        Self::AlignBottom,
        Self::DistributeHorizontal,
        Self::DistributeVertical,
    ];

    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::WrapText => "wrap-text",
            Self::Duplicate => "duplicate",
            Self::Copy => "copy",
            Self::Cut => "cut",
            Self::Paste => "paste",
            Self::CopyStyle => "copy-style",
            Self::PasteStyle => "paste-style",
            Self::Group => "group",
            Self::Ungroup => "ungroup",
            Self::Lock => "lock",
            Self::UnlockAll => "unlock-all",
            Self::FlipHorizontal => "flip-horizontal",
            Self::FlipVertical => "flip-vertical",
            Self::Backward => "backward",
            Self::ToBack => "to-back",
            Self::Forward => "forward",
            Self::ToFront => "to-front",
            Self::AlignLeft => "align-left",
            Self::AlignCentre => "align-centre",
            Self::AlignRight => "align-right",
            Self::AlignTop => "align-top",
            Self::AlignMiddle => "align-middle",
            Self::AlignBottom => "align-bottom",
            Self::DistributeHorizontal => "distribute-horizontal",
            Self::DistributeVertical => "distribute-vertical",
            Self::Delete => "delete",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::WrapText => "Wrap text in container",
            Self::Duplicate => "Duplicate",
            Self::Copy => "Copy",
            Self::Cut => "Cut",
            Self::Paste => "Paste",
            Self::CopyStyle => "Copy styles",
            Self::PasteStyle => "Paste styles",
            Self::Group => "Group",
            Self::Ungroup => "Ungroup",
            Self::Lock => "Lock / unlock selection",
            Self::UnlockAll => "Unlock all",
            Self::FlipHorizontal => "Flip horizontal",
            Self::FlipVertical => "Flip vertical",
            Self::Backward => "Send backward",
            Self::ToBack => "Send to back",
            Self::Forward => "Bring forward",
            Self::ToFront => "Bring to front",
            Self::AlignLeft => "Align left",
            Self::AlignCentre => "Align centre",
            Self::AlignRight => "Align right",
            Self::AlignTop => "Align top",
            Self::AlignMiddle => "Align middle",
            Self::AlignBottom => "Align bottom",
            Self::DistributeHorizontal => "Distribute horizontally",
            Self::DistributeVertical => "Distribute vertically",
            Self::Delete => "Delete",
        }
    }

    #[must_use]
    pub fn shortcut(self) -> &'static str {
        match self {
            Self::Duplicate => "Ctrl+D",
            Self::Copy => "Ctrl+C",
            Self::Cut => "Ctrl+X",
            Self::Paste => "Ctrl+V",
            Self::CopyStyle => "Ctrl+Alt+C",
            Self::PasteStyle => "Ctrl+Alt+V",
            Self::Group => "Ctrl+G",
            Self::Ungroup => "Ctrl+Shift+G",
            Self::Lock => "Ctrl+Shift+L",
            Self::FlipHorizontal => "Shift+H",
            Self::FlipVertical => "Shift+V",
            Self::Backward => "Ctrl+[",
            Self::ToBack => "Ctrl+Shift+[",
            Self::Forward => "Ctrl+]",
            Self::ToFront => "Ctrl+Shift+]",
            Self::Delete => "Delete",
            _ => "",
        }
    }

    #[must_use]
    pub fn for_key(key: Key, mods: Modifiers) -> Option<Self> {
        Self::ALL.into_iter().find(|a| {
            let shortcut = a.shortcut();
            if shortcut.is_empty() {
                return false;
            }
            let last = shortcut.rsplit('+').next().unwrap_or("");
            let matches = if last == "Delete" {
                matches!(key, Key::Delete | Key::Backspace)
            } else {
                matches!(key, Key::Char(c) if last.eq_ignore_ascii_case(&c.to_string()))
            };
            matches
                && mods.ctrl == shortcut.contains("Ctrl+")
                && mods.alt == shortcut.contains("Alt+")
                && mods.shift == shortcut.contains("Shift+")
        })
    }

    #[must_use]
    pub fn enabled(self, editor: &Editor) -> bool {
        let count = editor.selection().len();
        if editor.text_session().is_some()
            || editor.gesture.is_some()
            || editor.eraser.is_some()
            || editor.property_edit.is_some()
        {
            return false;
        }
        if !matches!(
            self,
            Self::Copy | Self::CopyStyle | Self::Lock | Self::UnlockAll | Self::Paste
        ) && editor
            .selection
            .elements(&editor.document)
            .any(|e| e.locked)
        {
            return false;
        }
        match self {
            Self::WrapText => {
                count == 1
                    && editor.selection.elements(&editor.document).all(|e| {
                        matches!(
                            e.kind,
                            pen_core::Kind::Text {
                                container: None,
                                ..
                            }
                        )
                    })
            }
            Self::Paste => true,
            Self::UnlockAll => editor.document().elements().iter().any(|e| e.locked),
            Self::Ungroup => editor
                .selection
                .elements(&editor.document)
                .any(|e| crate::selection::active_group(e, editor.group_scope).is_some()),
            Self::DistributeHorizontal | Self::DistributeVertical => editor.arrange_count() >= 3,
            Self::Group => count >= 2,
            Self::AlignLeft
            | Self::AlignCentre
            | Self::AlignRight
            | Self::AlignTop
            | Self::AlignMiddle
            | Self::AlignBottom => editor.arrange_count() >= 2,
            Self::PasteStyle => count > 0 && editor.copied_style.is_some(),
            Self::Copy | Self::CopyStyle | Self::Lock => count > 0,
            _ => {
                count > 0
                    && editor
                        .selection
                        .elements(&editor.document)
                        .all(|e| !e.locked)
            }
        }
    }

    pub fn run(self, editor: &mut Editor) -> Redraw {
        editor.handle(InputEvent::Action(self))
    }
}
