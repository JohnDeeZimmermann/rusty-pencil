use super::choices;
use crate::design as d;
use pen_core::{Edges, Property, StrokeStyle};
use pen_editor::Editor;
use rune::{El, col};
pub(super) fn group(editor: &Editor) -> El {
    let dark = editor.settings().dark;
    col()
        .gap(d::S3)
        .child(choices(
            "Stroke width",
            "width",
            editor.property(|p| p.stroke_width),
            &[("Thin", 1.0), ("Bold", 2.0), ("Extra", 4.0)],
            Property::Width,
            dark,
        ))
        .child(choices(
            "Stroke style",
            "stroke-style",
            editor.property(|p| p.stroke_style),
            &[
                ("Solid", StrokeStyle::Solid),
                ("Dash", StrokeStyle::Dashed),
                ("Dot", StrokeStyle::Dotted),
            ],
            Property::StrokeStyle,
            dark,
        ))
        .child(choices(
            "Edges",
            "edges",
            editor.property(|p| p.edges),
            &[("Sharp", Edges::Sharp), ("Round", Edges::Round)],
            Property::Edges,
            dark,
        ))
}
