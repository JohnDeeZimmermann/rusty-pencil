use super::choices;
use pen_core::{FillStyle, Property};
use pen_editor::Editor;
use rune::El;
pub(super) fn group(editor: &Editor) -> El {
    choices(
        "Fill",
        "fill",
        editor.property(|p| p.fill),
        &[
            ("Hatch", FillStyle::Hachure),
            ("Cross", FillStyle::CrossHatch),
            ("Solid", FillStyle::Solid),
        ],
        Property::Fill,
        editor.settings().dark,
    )
}
