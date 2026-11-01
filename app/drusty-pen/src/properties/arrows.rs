use super::{choice, group as field};
use crate::design as d;
use pen_core::{Arrowhead as A, Property, Value};
use pen_editor::{Editor, Tool};
use rune::{El, col, el, row};
pub(super) fn group(editor: &Editor) -> El {
    if editor.tool() != Tool::Arrow
        && !editor
            .selection()
            .ids()
            .filter_map(|id| editor.document().get(id))
            .any(|e| matches!(e.kind, pen_core::Kind::Arrow(_)))
    {
        return el().size(0.0, 0.0);
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
    let mut root = col().gap(d::S3);
    for (start, label) in [(true, "Start arrowhead"), (false, "End arrowhead")] {
        let value = editor.property(|p| if start { p.start_head } else { p.end_head });
        let controls = options.chunks(5).fold(col().gap(d::S1), |col, chunk| {
            col.child(chunk.iter().fold(row().gap(d::S1), |row, (label, head)| {
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
                        dark,
                    )
                    .help(format!("{head:?}"))
                    .min_w(d::CONTROL)
                    .w(d::CONTROL),
                )
            }))
        });
        root = root.child(field(label, value == Value::Mixed, controls, dark));
    }
    root
}
