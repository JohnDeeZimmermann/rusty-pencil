use super::*;
use crate::selection_tests::{CTRL, NONE, click, drag, key, p};
use pen_core::{
    Arrowhead, Edges, FillStyle, Property, Settings, Srgba, StrokeStyle, Value, WorldPt,
};

fn three() -> Editor {
    let mut editor = Editor::default();
    key(&mut editor, Key::Char('q'), NONE);
    for i in 0..3 {
        let x = i as f32 * 150.0;
        drag(&mut editor, p(x, 0.0), p(x + 100.0, 100.0), NONE);
    }
    key(&mut editor, Key::Char('a'), CTRL);
    editor
}

#[test]
fn changing_three_selected_elements_is_one_undo_step_and_preserves_mixed_fields() {
    let mut editor = three();
    let originals = editor.document.elements().to_vec();
    editor.handle(InputEvent::Property(Property::Stroke(Srgba::rgb(0x1971c2))));
    assert!(
        editor
            .document
            .elements()
            .iter()
            .all(|e| e.props.stroke == Srgba::rgb(0x1971c2))
    );
    key(&mut editor, Key::Char('z'), CTRL);
    assert!(
        editor
            .document
            .elements()
            .iter()
            .zip(originals.iter())
            .all(|(a, b)| a.props == b.props)
    );
    assert_eq!(editor.selection.len(), 3);
    key(&mut editor, Key::Char('y'), CTRL);
    assert!(
        editor
            .document
            .elements()
            .iter()
            .all(|e| e.props.stroke == Srgba::rgb(0x1971c2))
    );
    click(&mut editor, p(800.0, 300.0), NONE);
    click(&mut editor, p(50.0, 0.0), NONE);
    editor.handle(InputEvent::Property(Property::Width(4.0)));
    key(&mut editor, Key::Char('a'), CTRL);
    assert_eq!(editor.property(|p| p.stroke_width), Value::Mixed);
    editor.handle(InputEvent::Property(Property::Fill(FillStyle::CrossHatch)));
    assert_eq!(editor.property(|p| p.stroke_width), Value::Mixed);
    assert_eq!(
        editor.property(|p| p.fill),
        Value::Uniform(FillStyle::CrossHatch)
    );
}

#[test]
fn opacity_gesture_previews_only_touched_elements_and_commits_once_or_cancels() {
    let mut editor = three();
    let revision = editor.document.revision();
    editor.handle(InputEvent::BeginProperty);
    for opacity in [90, 75, 40] {
        editor.handle(InputEvent::Property(Property::Opacity(opacity)));
    }
    assert_eq!(editor.document.revision(), revision);
    assert_eq!(editor.preview().len(), 3);
    assert!(editor.preview().iter().all(|e| e.props.opacity == 40));
    editor.handle(InputEvent::EndProperty { cancel: false });
    assert!(editor.preview().is_empty());
    assert!(
        editor
            .document
            .elements()
            .iter()
            .all(|e| e.props.opacity == 40)
    );
    key(&mut editor, Key::Char('z'), CTRL);
    assert!(
        editor
            .document
            .elements()
            .iter()
            .all(|e| e.props.opacity == 100)
    );
    let before = editor.style;
    editor.handle(InputEvent::BeginProperty);
    editor.handle(InputEvent::Property(Property::Opacity(10)));
    editor.handle(InputEvent::Cancel);
    assert_eq!(editor.style, before);
    assert!(editor.preview().is_empty());
}

#[test]
fn new_elements_remember_all_properties_and_tool_lock_controls_return_to_selection() {
    let mut editor = Editor::default();
    for property in [
        Property::Background(Some(Srgba::rgb(0xffe3e3))),
        Property::Fill(FillStyle::Hachure),
        Property::StrokeStyle(StrokeStyle::Dashed),
        Property::Edges(Edges::Sharp),
        Property::Opacity(60),
        Property::StartHead(Arrowhead::CircleOutline),
        Property::EndHead(Arrowhead::Diamond),
    ] {
        editor.handle(InputEvent::Property(property));
    }
    let props = editor.style;
    drag(&mut editor, p(0.0, 0.0), p(100.0, 100.0), NONE);
    assert_eq!(editor.tool, Tool::Select);
    assert_eq!(editor.selection.len(), 1);
    assert_eq!(editor.document.elements()[0].props, props);
    let selected = editor.selection.clone();
    key(&mut editor, Key::Char('z'), CTRL);
    assert!(editor.selection.is_empty());
    key(&mut editor, Key::Char('y'), CTRL);
    assert_eq!(editor.selection, selected);
    key(&mut editor, Key::Char('q'), NONE);
    editor.handle(InputEvent::SelectTool(Tool::Arrow));
    drag(&mut editor, p(150.0, 0.0), p(250.0, 100.0), NONE);
    assert_eq!(editor.tool, Tool::Arrow);
    assert_eq!(editor.document.elements()[1].props, props);
    assert!(editor.selection.is_empty());
}

#[test]
fn grid_snaps_live_shapes_paths_moves_resizes_and_nudges_in_world_units() {
    for zoom in [0.5, 1.0, 4.0] {
        let mut editor = Editor::new(Camera::new(WorldPt::ORIGIN, zoom));
        editor.handle(InputEvent::Settings(Settings {
            grid: true,
            ..Default::default()
        }));
        drag(
            &mut editor,
            p(11.0 * zoom, 9.0 * zoom),
            p(111.0 * zoom, 89.0 * zoom),
            NONE,
        );
        let bounds = editor.document.elements()[0].kind.bounds();
        assert_eq!(bounds.min(), WorldPt::new(20.0, 0.0));
        assert_eq!(bounds.max(), WorldPt::new(120.0, 80.0));
        drag(
            &mut editor,
            p(60.0 * zoom, 40.0 * zoom),
            p(87.0 * zoom, 53.0 * zoom),
            NONE,
        );
        assert_eq!(
            editor.document.elements()[0].kind.bounds().min(),
            WorldPt::new(40.0, 20.0)
        );
        key(&mut editor, Key::Right, NONE);
        assert_eq!(
            editor.document.elements()[0].kind.bounds().min(),
            WorldPt::new(60.0, 20.0)
        );
        // The east handle sits five screen points beyond the selected bounds.
        drag(
            &mut editor,
            p(160.0 * zoom + 5.0, 60.0 * zoom),
            p(187.0 * zoom + 5.0, 60.0 * zoom),
            NONE,
        );
        assert_eq!(
            editor.document.elements()[0].kind.bounds().max(),
            WorldPt::new(180.0, 100.0)
        );
        editor.handle(InputEvent::SelectTool(Tool::Arrow));
        drag(
            &mut editor,
            p(211.0 * zoom, 9.0 * zoom),
            p(311.0 * zoom, 89.0 * zoom),
            NONE,
        );
        let points: Vec<_> = editor.document.elements()[1]
            .kind
            .path()
            .unwrap()
            .world_points()
            .collect();
        assert_eq!(
            points,
            [WorldPt::new(220.0, 0.0), WorldPt::new(320.0, 80.0)]
        );
    }
}

#[test]
fn theme_and_grid_shortcuts_preserve_document_colours_and_history() {
    let mut editor = three();
    let original = editor.document.elements().to_vec();
    key(
        &mut editor,
        Key::Char('d'),
        Modifiers {
            alt: true,
            shift: true,
            ..NONE
        },
    );
    key(&mut editor, Key::Char('\''), CTRL);
    assert!(editor.settings.dark && editor.settings.grid);
    assert_eq!(editor.document.elements(), original);
    key(&mut editor, Key::Char('z'), CTRL);
    assert_eq!(editor.document.elements().len(), 2);
}

#[test]
fn cancelling_opacity_while_typing_restores_the_session_and_its_defaults() {
    let mut editor = Editor::default();
    editor.handle(InputEvent::SelectTool(Tool::Text));
    click(&mut editor, p(100.0, 100.0), NONE);
    let original = editor.text_session().unwrap().element.props;
    let defaults = editor.style();
    editor.handle(InputEvent::BeginProperty);
    editor.handle(InputEvent::Property(Property::Opacity(25)));
    assert_eq!(editor.text_session().unwrap().element.props.opacity, 25);
    editor.handle(InputEvent::EndProperty { cancel: true });
    assert_eq!(editor.text_session().unwrap().element.props, original);
    assert_eq!(editor.style(), defaults);
}
