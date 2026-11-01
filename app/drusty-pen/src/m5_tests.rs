use super::tests::{FRAME, app, drag, key};
use super::*;
use pen_core::{FillStyle, Srgba, StrokeStyle, Value};
use rune::{Runtime, WheelEvent};

fn click(rt: &mut Runtime<State>, name: &str) {
    let (x, y) = rt.find(name).expect(name);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(FRAME);
}
fn reveal(rt: &mut Runtime<State>, name: &str) {
    for _ in 0..8 {
        let (_, y) = rt.find(name).expect(name);
        if (155.0..585.0).contains(&y) {
            return;
        }
        let dy = if y > 585.0 { -180.0 } else { 180.0 };
        rt.wheel_event(WheelEvent::new(0.0, dy).at(210.0, 550.0));
        for _ in 0..60 {
            rt.frame(FRAME);
        }
    }
    panic!("could not scroll to {name}: {:?}", rt.find(name));
}

#[test]
fn panel_colours_hex_fill_and_stroke_controls_edit_selection_through_real_input() {
    let mut rt = app();
    drag(&mut rt, (300.0, 250.0), (500.0, 400.0));
    click(&mut rt, "Stroke-color-3");
    click(&mut rt, "Background-color-1");
    click(&mut rt, "fill-1");
    let props = rt.state.editor.document().elements()[0].props;
    assert_eq!(props.stroke, Srgba::rgb(0x1971c2));
    assert_eq!(props.background, Some(Srgba::rgb(0xd3f9d8)));
    assert_eq!(props.fill, FillStyle::CrossHatch);
    click(&mut rt, "Stroke-hex");
    rt.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            cmd: true,
            ..Default::default()
        },
    });
    key(&mut rt, "#8750aa");
    rt.key(KeyEvent {
        key: Key::Enter,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert_eq!(
        rt.state.editor.document().elements()[0].props.stroke,
        Srgba::rgb(0x8750aa)
    );
    reveal(&mut rt, "stroke-style-1");
    click(&mut rt, "stroke-style-1");
    assert_eq!(
        rt.state.editor.document().elements()[0].props.stroke_style,
        StrokeStyle::Dashed
    );
    reveal(&mut rt, "opacity");
    let (x, y) = rt.find("opacity").unwrap();
    rt.pointer_move(x - 70.0, y);
    rt.pointer_down(x - 70.0, y);
    rt.frame(FRAME);
    assert!(
        !rt.state.editor.preview().is_empty(),
        "slider down at {x},{y}, props {:?}",
        rt.state.editor.property(|p| p.opacity)
    );
    rt.pointer_move(x + 20.0, y);
    rt.frame(FRAME);
    assert!(
        rt.state.editor.preview()[0].props.opacity < 100,
        "preview at {x},{y}: {:?}",
        rt.state.editor.preview()
    );
    rt.pointer_up(x + 20.0, y);
    rt.frame(FRAME);
    assert!(rt.state.editor.document().elements()[0].props.opacity < 100);
    rt.key(KeyEvent {
        key: Key::Command('z'),
        mods: Mods {
            cmd: true,
            ..Default::default()
        },
    });
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.document().elements()[0].props.opacity, 100);
}

#[test]
fn theme_grid_and_canvas_settings_keep_camera_updates_live_and_document_unchanged() {
    let mut rt = app();
    drag(&mut rt, (300.0, 250.0), (500.0, 400.0));
    let stored = rt.state.editor.document().elements().to_vec();
    key(&mut rt, "v");
    // The inspector scrolls independently; controls never pan the document.
    let camera = *rt.state.editor.camera();
    reveal(&mut rt, "grid-toggle");
    click(&mut rt, "grid-toggle");
    reveal(&mut rt, "dark-toggle");
    click(&mut rt, "dark-toggle");
    assert_eq!(*rt.state.editor.camera(), camera);
    assert!(rt.state.editor.settings().dark && rt.state.editor.settings().grid);
    assert_eq!(rt.state.editor.document().elements(), stored);
    let commits = rt.stats.commits;
    rt.wheel_event(WheelEvent::new(3.0, 7.0).at(700.0, 500.0));
    rt.frame(FRAME);
    rt.magnify_event(rune::MagnifyEvent::new(0.2).at(700.0, 500.0));
    rt.frame(FRAME);
    assert_eq!(rt.stats.commits, commits);
    reveal(&mut rt, "Canvas-color-2");
    click(&mut rt, "Canvas-color-2");
    assert_eq!(rt.state.editor.settings().canvas, Srgba::rgb(0xfff9db));
    assert_eq!(rt.state.editor.document().elements(), stored);
    assert_eq!(
        rt.state.editor.property(|p| p.stroke),
        Value::Uniform(stored[0].props.stroke)
    );
}

#[test]
fn toggling_dark_mode_repaints_cached_scene_nodes_without_editing_the_document() {
    let mut rt = app();
    click(&mut rt, "Stroke-color-1");
    drag(&mut rt, (300.0, 250.0), (500.0, 400.0));
    let revision = rt.state.editor.document().revision();
    let original = rt.state.editor.document().elements()[0].props.stroke;
    let has_scene_color = |rt: &Runtime<State>, color: pen_core::Srgba| {
        let c = palette::color(color);
        rt.instances.iter().any(|i| {
            i.rect[0] > 570.0
                && i.rect[0] < 1030.0
                && i.rect[1] > 470.0
                && i.rect[1] < 830.0
                && (i.fill[0] - c.r).abs() < 0.001
                && (i.fill[1] - c.g).abs() < 0.001
                && (i.fill[2] - c.b).abs() < 0.001
        })
    };
    assert!(has_scene_color(&rt, original));
    for dark in [true, false] {
        rt.key(KeyEvent {
            key: Key::Text("d".into()),
            mods: Mods {
                alt: true,
                shift: true,
                ..Default::default()
            },
        });
        for _ in 0..60 {
            rt.frame(FRAME);
        }
        assert_eq!(rt.state.editor.settings().dark, dark);
        assert!(has_scene_color(
            &rt,
            if dark { original.dark() } else { original }
        ));
        assert_eq!(rt.state.editor.document().revision(), revision);
    }
}
