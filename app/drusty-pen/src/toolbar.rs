//! The floating tool island at the top of the window, and the quiet hint under it.

use pen_core::{Edges, Kind, Props, Srgba, WorldPt, WorldRect};
use pen_editor::{InputEvent, Tool};
use pen_render::DrawList;
use rune::{Color, Cursor, El, Tx, canvas, col, el, row, text};

use crate::design::{self as d, c};
use crate::{State, paint};

/// The island, centred at the top, and the active tool's hint below it.
pub(crate) fn toolbar(active: Tool) -> El {
    let island = Tool::ALL.into_iter().fold(
        d::raised(row().key("tools"), d::R_ISLAND)
            .padding(d::PADDING)
            .gap(d::S1)
            .cursor(Cursor::Default)
            // A press between buttons must not start drawing underneath the island.
            .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only()),
        |island, tool| island.child(button(tool, tool == active)),
    );
    col()
        .key("toolbar")
        .pin(Some(0.0), Some(d::S4), Some(0.0), None)
        .items_center()
        .gap(d::S3)
        .child(island)
        .child(
            text(hint(active))
                .key("hint")
                .font(d::T_HINT)
                .color(c(d::MUTED)),
        )
}

fn button(tool: Tool, active: bool) -> El {
    let ink = if active { d::ON_ACCENT } else { d::INK };
    el().key(format!("tool-{}", tool.letter()))
        .size(d::BUTTON, d::BUTTON)
        .radius(d::R_BUTTON)
        .fill(if active {
            c(d::ACCENT)
        } else {
            Color::TRANSPARENT
        })
        .hover(if active {
            Color::TRANSPARENT
        } else {
            c(d::ACCENT_SOFT)
        })
        .press_scale(d::PRESS)
        .items_center()
        .justify_center()
        .child(canvas(icon(tool, ink)).size(d::ICON, d::ICON))
        .child(shortcut(tool.digit(), active))
        .help(format!(
            "{} — {} or {}",
            tool.name(),
            tool.letter().to_ascii_uppercase(),
            tool.digit()
        ))
        .on_press_tx(move |s: &mut State, tx| {
            s.input(InputEvent::SelectTool(tool), tx);
        })
}

/// Shortcuts stay inside the hit target and follow the selected tool's colour.
fn shortcut(digit: char, active: bool) -> El {
    text(digit.to_string())
        .size(d::SHORTCUT, d::SHORTCUT)
        .font(d::T_BADGE)
        .color(c(if active { d::ON_ACCENT } else { d::MUTED }))
        .pin(None, None, Some(d::SHORTCUT_INSET), Some(d::SHORTCUT_INSET))
}

/// Tool icons are the shapes themselves, drawn by the same renderer as the canvas.
fn icon(tool: Tool, ink: Srgba) -> rune::Shape {
    let inset = |e: f32| {
        WorldRect::from_corners(WorldPt::new(e, e), WorldPt::new(d::ICON - e, d::ICON - e))
    };
    let kind = match tool {
        Tool::Rectangle => Kind::Rect(inset(2.0)),
        Tool::Diamond => Kind::Diamond(inset(1.0)),
        Tool::Ellipse => Kind::Ellipse(inset(2.0)),
    };
    let props = Props {
        stroke: ink,
        stroke_width: d::ICON_STROKE,
        edges: Edges::Round,
        ..Props::default()
    };
    let mut list = DrawList::new();
    pen_render::draw(&kind, &props, &mut list);
    paint::shape(list.prims())
}

fn hint(tool: Tool) -> String {
    let what = match tool {
        Tool::Rectangle => "a rectangle",
        Tool::Diamond => "a diamond",
        Tool::Ellipse => "an ellipse",
    };
    let square = if tool == Tool::Ellipse {
        "circle"
    } else {
        "square"
    };
    format!("Drag to draw {what}  ·  Shift for a {square}  ·  Alt from the centre")
}
