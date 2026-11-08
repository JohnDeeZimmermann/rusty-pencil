use super::*;
use pen_core::{Arrowhead, Edges, FillStyle, Path, StrokeStyle, WorldRect};

fn rect() -> Kind {
    Kind::Rect(WorldRect::from_corners(
        WorldPt::ORIGIN,
        WorldPt::new(100.0, 80.0),
    ))
}

#[test]
fn dash_phase_survives_corners_and_lengths_are_world_units_at_every_zoom() {
    let points = [
        WorldPt::new(0.0, 0.0),
        WorldPt::new(5.0, 0.0),
        WorldPt::new(5.0, 25.0),
    ];
    let segments = style::pattern(&points, false, 8.0, 6.0);
    assert_eq!(segments[0], [points[0], points[1], WorldPt::new(5.0, 3.0)]);
    let path = Kind::Line(Path::from_world(&[
        WorldPt::ORIGIN,
        WorldPt::new(80.0, 0.0),
    ]));
    let props = Props {
        stroke_style: StrokeStyle::Dashed,
        roughness: pen_core::Roughness::Architect,
        ..Default::default()
    };
    let mut expected = DrawList::new();
    draw(&path, &props, &mut expected);
    for zoom in [0.1, 1.0, 4.0, 30.0] {
        let mut editor = pen_editor::Editor::default();
        editor.handle(pen_editor::InputEvent::Zoom {
            about: pen_editor::ScreenPt::new(0.0, 0.0),
            factor: zoom,
        });
        editor.handle(pen_editor::InputEvent::Property(
            pen_core::Property::StrokeStyle(StrokeStyle::Dashed),
        ));
        editor.handle(pen_editor::InputEvent::Roughness(
            pen_core::Roughness::Architect,
        ));
        editor.handle(pen_editor::InputEvent::SelectTool(pen_editor::Tool::Line));
        editor.handle(pen_editor::InputEvent::PointerDown {
            at: pen_editor::ScreenPt::new(0.0, 0.0),
            mods: pen_editor::Modifiers::NONE,
        });
        editor.handle(pen_editor::InputEvent::PointerUp {
            at: pen_editor::ScreenPt::new(80.0 * zoom, 0.0),
            mods: pen_editor::Modifiers::NONE,
        });
        let mut actual = DrawList::new();
        draw_element(&editor.document().elements()[0], &mut actual);
        assert_eq!(actual, expected);
        for prim in actual.prims() {
            if let Prim::Stroke { points, .. } = prim {
                let length = (points.last().unwrap().x - points[0].x).abs();
                assert!(
                    (length - 8.0).abs() < 0.001,
                    "{length} world points at {zoom}"
                );
            }
        }
    }
}

#[test]
fn hatch_and_cross_hatch_are_clipped_to_the_shape_and_opacity_reaches_every_primitive() {
    for fill in [FillStyle::Solid, FillStyle::Hachure, FillStyle::CrossHatch] {
        for kind in [
            rect(),
            Kind::Ellipse(rect().bounds()),
            Kind::Diamond(rect().bounds()),
        ] {
            let props = Props {
                background: Some(Srgba::rgb(0x1971c2)),
                fill,
                opacity: 40,
                edges: Edges::Sharp,
                ..Default::default()
            };
            let mut list = DrawList::new();
            draw(&kind, &props, &mut list);
            assert!(list.prims().len() >= 2);
            for prim in list.prims() {
                let (Prim::Fill { points, color } | Prim::Stroke { points, color, .. }) = prim
                else {
                    panic!("expected geometry");
                };
                assert_eq!(color.a, 102);
                assert!(points.iter().all(|p| p.is_finite()
                    && p.x >= -0.001
                    && p.x <= 100.001
                    && p.y >= -0.001
                    && p.y <= 80.001));
            }
        }
    }
}

#[test]
fn every_arrowhead_renders_at_both_ends_and_degenerate_paths_stay_finite() {
    for head in [
        Arrowhead::None,
        Arrowhead::Arrow,
        Arrowhead::Bar,
        Arrowhead::Dot,
        Arrowhead::Triangle,
        Arrowhead::TriangleOutline,
        Arrowhead::Circle,
        Arrowhead::CircleOutline,
        Arrowhead::Diamond,
        Arrowhead::DiamondOutline,
    ] {
        for points in [
            vec![WorldPt::ORIGIN],
            vec![
                WorldPt::ORIGIN,
                WorldPt::new(100.0, 0.0),
                WorldPt::new(100.0, 0.0),
            ],
        ] {
            let kind = Kind::Arrow(Path::from_world(&points));
            let props = Props {
                start_head: head,
                end_head: head,
                stroke_style: StrokeStyle::Dotted,
                opacity: 50,
                ..Default::default()
            };
            let mut list = DrawList::new();
            draw_seeded(&kind, &props, 42, &mut list);
            for prim in list.prims() {
                let (Prim::Fill { points, .. } | Prim::Stroke { points, .. }) = prim else {
                    panic!("expected geometry");
                };
                assert!(points.iter().all(|p| p.is_finite()));
            }
            if points.len() > 1 && head != Arrowhead::None {
                assert!(list.prims().len() > 20);
            }
        }
    }
}

fn segments(list: &DrawList) -> usize {
    list.prims()
        .iter()
        .map(|prim| match prim {
            Prim::Stroke { points, closed, .. } => points.len() - usize::from(!closed),
            _ => 0,
        })
        .sum()
}

#[test]
fn rough_dashes_cut_the_wobbled_contour_without_resampling_each_dash() {
    // Wobbling each dash on its own resampled an 8 pt dash into hundreds of points: a 480 x 360
    // dashed ellipse painted 15,831 Rune instances (dotted 19,872) against 1,778 for solid.
    let kind = Kind::Ellipse(WorldRect::from_corners(
        WorldPt::ORIGIN,
        WorldPt::new(480.0, 360.0),
    ));
    for roughness in [pen_core::Roughness::Artist, pen_core::Roughness::Cartoonist] {
        let paint = |stroke_style| {
            let props = Props {
                stroke_style,
                roughness,
                ..Default::default()
            };
            let mut list = DrawList::new();
            draw_seeded(&kind, &props, 99, &mut list);
            list
        };
        let solid = paint(StrokeStyle::Solid);
        let Some(Prim::Stroke {
            points: contour, ..
        }) = solid.prims().first()
        else {
            panic!("expected the solid outline");
        };
        for style in [StrokeStyle::Dashed, StrokeStyle::Dotted] {
            let styled = paint(style);
            assert!(styled.prims().len() > 50, "{style:?} has visible dashes");
            assert!(
                segments(&styled) <= segments(&solid),
                "{style:?}: {} segments, solid {}",
                segments(&styled),
                segments(&solid)
            );
            // Every dash point lies on the solid wobble, so styles share one hand-drawn line.
            for prim in styled.prims() {
                let Prim::Stroke { points, .. } = prim else {
                    continue;
                };
                for p in points {
                    let near = contour
                        .iter()
                        .zip(contour.iter().cycle().skip(1))
                        .any(|(a, b)| distance_to_segment(*p, *a, *b) < 0.001);
                    assert!(near, "{style:?} point {p:?} is off the wobbled contour");
                }
            }
        }
    }
}

fn distance_to_segment(p: WorldPt, a: WorldPt, b: WorldPt) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - a.x - dx * t).hypot(p.y - a.y - dy * t)
}
