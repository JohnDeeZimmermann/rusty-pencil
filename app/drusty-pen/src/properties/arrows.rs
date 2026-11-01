use super::{choice, group as field, segment_w, track};
use crate::design as d;
use pen_core::{Arrowhead as A, Property, Value};
use pen_editor::{Editor, Tool};
use rune::{El, col, row};
pub(super) fn group(editor: &Editor) -> Option<El> {
    if editor.tool() != Tool::Arrow
        && !editor
            .selection()
            .ids()
            .filter_map(|id| editor.document().get(id))
            .any(|e| matches!(e.kind, pen_core::Kind::Arrow(_)))
    {
        return None;
    }
    let dark = editor.settings().dark;
    let options = [
        ("—", A::None),
        ("›", A::Arrow),
        ("⊣", A::Bar),
        ("•", A::Dot),
        ("▲", A::Triangle),
        ("△", A::TriangleOutline),
        ("●", A::Circle),
        ("○", A::CircleOutline),
        ("◆", A::Diamond),
        ("◇", A::DiamondOutline),
    ];
    let value = editor.property(|p| p.arrow_type);
    let types = [
        ("Sharp", pen_core::ArrowType::Sharp),
        ("Curved", pen_core::ArrowType::Curved),
        ("Elbow", pen_core::ArrowType::Elbow),
    ]
    .into_iter()
    .fold(row(), |r, (label, t)| {
        r.child(choice(
            &format!("arrow-type-{t:?}"),
            label,
            value == Value::Uniform(t),
            Property::ArrowType(t),
            (segment_w(3), d::T_CONTROL),
            dark,
        ))
    });
    let mut root = col().gap(d::S_SECTION).child(field(
        "Arrow type",
        value == Value::Mixed,
        track(types, dark),
        dark,
    ));
    let width = segment_w(5);
    for (start, label) in [(true, "Start arrowhead"), (false, "End arrowhead")] {
        let value = editor.property(|p| if start { p.start_head } else { p.end_head });
        let controls = options.chunks(5).fold(col(), |col, chunk| {
            col.child(
                chunk
                    .iter()
                    .fold(row().gap(d::SEGMENT_INSET), |row, (label, head)| {
                        row.child(
                            choice(
                                &format!("head-{start}-{head:?}"),
                                label,
                                value == Value::Uniform(*head),
                                if start {
                                    Property::StartHead(*head)
                                } else {
                                    Property::EndHead(*head)
                                },
                                (width, d::T_GLYPH),
                                dark,
                            )
                            .help(format!("{head:?}")),
                        )
                    }),
            )
        });
        root = root.child(field(
            label,
            value == Value::Mixed,
            track(controls, dark),
            dark,
        ));
    }
    Some(root)
}
