//! The floating tool island at the top of the window, and the hint under it.

use pen_core::{Edges, Kind, Props, Srgba, WorldPt, WorldRect};
use pen_editor::{InputEvent, Tool};
use pen_render::DrawList;
use rune::tok::TextStyle;
use rune::{Color, Cursor, El, Tx, canvas, col, el, row, text};

use crate::palette::{self, ui};
use crate::{State, paint};

const BUTTON: f32 = 36.0;
const ICON: f32 = 18.0;
const DIGIT: TextStyle = TextStyle::new(9.0, 500.0);
const HINT: TextStyle = TextStyle::new(12.0, 400.0);

/// The island, centred at the top, and the active tool's hint below it.
pub(crate) fn toolbar(active: Tool) -> El {
    let island = Tool::ALL.into_iter().fold(
        row()
            .key("tools")
            .padding(4.0)
            .gap(2.0)
            .radius(10.0)
            .fill(ui::island())
            .border(1.0, ui::island_border())
            .shadow(1.0, 4.0, ui::shadow())
            .shadow(6.0, 18.0, ui::shadow())
            .cursor(Cursor::Default)
            // A press between buttons must not start drawing underneath the island.
            .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only()),
        |island, tool| island.child(button(tool, tool == active)),
    );
    col()
        .key("toolbar")
        .pin(Some(0.0), Some(14.0), Some(0.0), None)
        .items_center()
        .gap(10.0)
        .child(island)
        .child(
            text(hint(active))
                .key("hint")
                .font(HINT)
                .color(palette::color(ui::CAPTION)),
        )
}

fn button(tool: Tool, active: bool) -> El {
    let ink = if active { ui::ACTIVE_INK } else { ui::ICON_INK };
    el().key(format!("tool-{}", tool.letter()))
        .size(BUTTON, BUTTON)
        .radius(8.0)
        .fill(if active {
            ui::active()
        } else {
            Color::TRANSPARENT
        })
        .hover(if active {
            Color::TRANSPARENT
        } else {
            ui::hover()
        })
        .press(ui::press())
        .items_center()
        .justify_center()
        .child(canvas(icon(tool, ink)).size(ICON, ICON))
        .child(
            text(tool.digit().to_string())
                .font(DIGIT)
                .color(palette::color(if active {
                    ui::ACTIVE_INK
                } else {
                    ui::CAPTION
                }))
                .pin(None, None, Some(3.0), Some(1.0)),
        )
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

/// Tool icons are the shapes themselves, drawn by the same renderer as the canvas.
fn icon(tool: Tool, ink: Srgba) -> rune::Shape {
    let inset =
        |d: f32| WorldRect::from_corners(WorldPt::new(d, d), WorldPt::new(ICON - d, ICON - d));
    let kind = match tool {
        Tool::Rectangle => Kind::Rect(inset(2.0)),
        Tool::Diamond => Kind::Diamond(inset(1.0)),
        Tool::Ellipse => Kind::Ellipse(inset(2.0)),
    };
    let props = Props {
        stroke: ink,
        stroke_width: 1.6,
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
    format!("Drag to draw {what}  ·  Shift for a {square}  ·  Alt to draw from the centre")
}
