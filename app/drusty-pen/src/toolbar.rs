//! The floating tool island at the top of the window, and the quiet hint under it.

use pen_core::{Edges, Kind, Path, Props, Roughness, Srgba, WorldPt, WorldRect};
use pen_editor::{Editor, InputEvent, Tool};
use pen_render::DrawList;
use rune::{Color, Cursor, El, Tx, canvas, col, el, row, text};

use crate::design::{self as d, c};
use crate::{State, paint};

/// The island, centred at the top, and the active tool's hint below it.
pub(crate) fn toolbar(editor: &Editor) -> El {
    let dark = editor.settings().dark;
    let c = |v| d::themed(v, dark);
    let active = editor.tool();
    let island = Tool::ALL.into_iter().fold(
        d::surface(row().key("tools"), d::R_ISLAND, dark)
            .padding(d::PADDING)
            .gap(d::S1)
            .cursor(Cursor::Default)
            // A press between buttons must not start drawing underneath the island.
            .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only()),
        |island, tool| island.child(button(tool, tool == active, dark)),
    );
    col()
        .key("toolbar")
        .pin(Some(0.0), Some(d::S4), Some(0.0), None)
        .items_center()
        .gap(d::S3)
        .child(island
            .child(el().size(d::S1, d::BUTTON).items_center().justify_center().child(el().size(d::OUTLINE, d::ICON).fill(c(d::BORDER))))
            .child(roughness_button(editor.roughness(), dark)))
        .child(
            text(if editor.editable_points().is_some() { "Drag a point to move  ·  Click a segment to add  ·  Delete removes a point  ·  Esc to finish".into() } else { hint(active) })
                .key("hint")
                .font(d::T_HINT)
                .color(c(d::MUTED)),
        )
}

fn button(tool: Tool, active: bool, dark: bool) -> El {
    let c = |v| d::themed(v, dark);
    let ink = if active { d::ON_ACCENT } else { d::INK };
    el().key(format!("tool-{}", tool.letter()))
        .preserve_focus_on_press(true)
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
        .child(canvas(icon(tool, if dark { ink.dark() } else { ink })).size(d::ICON, d::ICON))
        .child(shortcut(tool.digit().to_ascii_uppercase(), active, dark))
        .help(if tool == Tool::Hand {
            "Hand — H".into()
        } else {
            format!(
                "{} — {} or {}",
                tool.name(),
                tool.letter().to_ascii_uppercase(),
                tool.digit()
            )
        })
        .on_press_tx(move |s: &mut State, tx| {
            s.input(InputEvent::SelectTool(tool), tx);
        })
}

/// Shortcuts stay inside the hit target and follow the selected tool's colour.
fn shortcut(digit: char, active: bool, dark: bool) -> El {
    let c = |v| d::themed(v, dark);
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
        Tool::Text => {
            return rune::Shape::new()
                .stroke(vec![[3.0, 4.0], [17.0, 4.0]], d::ICON_STROKE, c(ink))
                .stroke(vec![[10.0, 4.0], [10.0, 17.0]], d::ICON_STROKE, c(ink));
        }
        Tool::Select => {
            let points = vec![
                [4.0, 2.0],
                [16.0, 10.0],
                [10.0, 11.0],
                [8.0, 17.0],
                [4.0, 2.0],
            ];
            return rune::Shape::new().stroke(points, d::ICON_STROKE, c(ink));
        }
        Tool::Hand => {
            return rune::Shape::new().stroke(
                vec![
                    [6.0, 11.0],
                    [6.0, 4.0],
                    [8.5, 4.0],
                    [8.5, 10.0],
                    [8.5, 2.0],
                    [11.0, 2.0],
                    [11.0, 10.0],
                    [11.0, 3.0],
                    [13.5, 3.0],
                    [13.5, 10.0],
                    [13.5, 5.0],
                    [16.0, 5.0],
                    [16.0, 12.0],
                    [14.0, 17.0],
                    [8.0, 17.0],
                    [3.0, 11.0],
                    [3.0, 9.0],
                    [6.0, 11.0],
                ],
                d::ICON_STROKE,
                c(ink),
            );
        }
        Tool::Arrow => Kind::Arrow(Path::from_world(&[
            WorldPt::new(2.0, 15.0),
            WorldPt::new(18.0, 5.0),
        ])),
        Tool::Line => Kind::Line(Path::from_world(&[
            WorldPt::new(3.0, 16.0),
            WorldPt::new(17.0, 4.0),
        ])),
        Tool::Freedraw => Kind::Line(Path::from_world(&[
            WorldPt::new(2.0, 15.0),
            WorldPt::new(6.0, 7.0),
            WorldPt::new(9.0, 6.0),
            WorldPt::new(10.0, 14.0),
            WorldPt::new(13.0, 15.0),
            WorldPt::new(18.0, 7.0),
        ])),
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
        Tool::Text => return "Click to write  ·  Enter for a new line  ·  Esc to finish  ·  Double-click to edit".into(),
        Tool::Select => return "Click to select  ·  Shift to add  ·  Drag to move or box select  ·  Double-click to write".into(),
        Tool::Hand => return "Drag to pan  ·  Wheel to scroll  ·  Ctrl + wheel to zoom  ·  Shift + 1 to fit all".into(),
        Tool::Line | Tool::Arrow => return "Drag or click point to point  ·  Shift snaps 15°  ·  Enter / Esc / double-click to finish".into(),
        Tool::Freedraw => return "Drag to draw freely  ·  Esc to cancel  ·  Space + drag to pan".into(),
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

/// Outline style remains available without opening an inspector on an empty canvas.
fn roughness_button(current: Roughness, dark: bool) -> El {
    let c = |v| d::themed(v, dark);
    let (name, next) = match current {
        Roughness::Architect => ("Architect", Roughness::Artist),
        Roughness::Artist => ("Artist", Roughness::Cartoonist),
        Roughness::Cartoonist => ("Cartoonist", Roughness::Architect),
    };
    el().key("roughness")
        .size(d::BUTTON, d::BUTTON)
        .radius(d::R_BUTTON)
        .items_center()
        .justify_center()
        .hover(c(d::ACCENT_SOFT))
        .press_scale(d::PRESS)
        .preserve_focus_on_press(true)
        .child(canvas(crate::property_icons::roughness(current, dark)).size(d::ICON, d::ICON))
        .help(format!(
            "Sloppiness: {name} · Click to cycle outline styles"
        ))
        .on_press_tx(move |s: &mut State, tx| {
            s.input(InputEvent::Roughness(next), tx);
        })
}
