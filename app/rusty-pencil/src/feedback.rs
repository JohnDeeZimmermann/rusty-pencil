//! Ephemeral world-space feedback follows the camera's live transform.
use crate::design as d;
use pen_core::{Kind, Srgba, WorldRect};
use pen_editor::Editor;
use pen_render::{DrawList, Prim};
pub(crate) fn shape(editor: &Editor) -> (rune::Shape, WorldRect) {
    let zoom = editor.camera().zoom();
    let mut list = DrawList::new();
    let ink = if editor.settings().dark {
        d::ON_ACCENT.dark()
    } else {
        d::ON_ACCENT
    };
    for (a, b) in editor.guides() {
        list.push(Prim::Stroke {
            points: vec![*a, *b],
            closed: false,
            width: d::OUTLINE / zoom,
            color: ink,
        });
    }
    if !editor.lasso().is_empty() {
        list.push(Prim::Fill {
            points: editor.lasso().to_vec(),
            color: Srgba {
                a: d::SELECTION_WASH.a,
                ..ink
            },
        });
        list.push(Prim::Stroke {
            points: editor.lasso().to_vec(),
            closed: true,
            width: d::OUTLINE / zoom,
            color: ink,
        });
    }
    for trail in editor.laser().windows(2) {
        let alpha = ((1.0 - trail[0].1 / 0.9).clamp(0.0, 1.0) * 255.0) as u8;
        list.push(Prim::Stroke {
            points: vec![trail[0].0, trail[1].0],
            closed: false,
            width: d::S1 / zoom,
            color: Srgba {
                a: alpha,
                ..d::LASER
            },
        });
    }
    if let Some((p, age)) = editor.laser().last() {
        let bounds = WorldRect::from_centre(*p, d::S1 / 2.0 / zoom, d::S1 / 2.0 / zoom);
        list.push(Prim::Fill {
            points: pen_core::outline(&Kind::Ellipse(bounds), pen_core::Edges::Sharp),
            color: Srgba {
                a: ((1.0 - age / 0.9).clamp(0.0, 1.0) * 255.0) as u8,
                ..d::LASER
            },
        });
    }
    crate::paint::bounded(list.prims())
}
