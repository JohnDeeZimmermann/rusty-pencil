use super::*;
fn p(x: f32, y: f32) -> ScreenPt {
    ScreenPt::new(x, y)
}
const NONE: Modifiers = Modifiers::NONE;
fn key(ed: &mut Editor, c: char, mods: Modifiers) -> Redraw {
    ed.handle(InputEvent::Key {
        key: Key::Char(c),
        mods,
    })
}
fn draw(ed: &mut Editor, from: ScreenPt, to: ScreenPt) {
    ed.handle(InputEvent::PointerDown {
        at: from,
        mods: NONE,
    });
    ed.handle(InputEvent::PointerUp { at: to, mods: NONE });
}

#[test]
fn hand_space_and_middle_pan_without_document_changes() {
    for mode in 0..3 {
        let mut ed = Editor::default();
        if mode == 0 {
            ed.handle(InputEvent::SelectTool(Tool::Hand));
        }
        if mode == 1 {
            ed.handle(InputEvent::Space(true));
        }
        ed.handle(if mode == 2 {
            InputEvent::PanStart { at: p(10.0, 20.0) }
        } else {
            InputEvent::PointerDown {
                at: p(10.0, 20.0),
                mods: NONE,
            }
        });
        ed.handle(if mode == 2 {
            InputEvent::PanMove { at: p(80.0, 60.0) }
        } else {
            InputEvent::PointerMove {
                at: p(80.0, 60.0),
                mods: NONE,
            }
        });
        assert_eq!(ed.camera.scroll(), pen_core::WorldPt::new(-70.0, -40.0));
        ed.handle(InputEvent::Space(false));
        ed.handle(if mode == 2 {
            InputEvent::PanEnd
        } else {
            InputEvent::PointerUp {
                at: p(80.0, 60.0),
                mods: NONE,
            }
        });
        assert!(!ed.is_panning());
        assert_eq!(ed.document.revision(), 0);
        assert!(ed.draft().is_none());
    }
}

#[test]
fn focus_cancel_releases_space_and_pan() {
    let mut ed = Editor::default();
    ed.handle(InputEvent::Space(true));
    ed.handle(InputEvent::PointerDown {
        at: p(0.0, 0.0),
        mods: NONE,
    });
    ed.handle(InputEvent::Cancel);
    assert!(!ed.is_panning() && !ed.space_held());
    draw(&mut ed, p(10.0, 10.0), p(100.0, 100.0));
    assert_eq!(ed.document.elements().len(), 1);
}

#[test]
fn zoom_shortcuts_anchor_viewport_centre_reset_and_keep_selection() {
    let mut ed = Editor::default();
    ed.handle(InputEvent::Viewport(ScreenVec::new(800.0, 600.0)));
    draw(&mut ed, p(10.0, 10.0), p(100.0, 100.0));
    let ctrl = Modifiers { ctrl: true, ..NONE };
    key(&mut ed, 'a', ctrl);
    let centre = p(400.0, 300.0);
    let fixed = ed.camera.screen_to_world(centre);
    key(&mut ed, '+', ctrl);
    assert!((ed.camera.zoom() - 1.2).abs() < 1e-5);
    assert_eq!(ed.camera.screen_to_world(centre), fixed);
    key(&mut ed, '-', ctrl);
    key(&mut ed, '0', ctrl);
    assert!((ed.camera.zoom() - 1.0).abs() < 1e-5);
    assert_eq!(ed.selection.len(), 1);
}

#[test]
fn content_button_detects_empty_gaps_and_refreshes_after_undo() {
    let mut ed = Editor::default();
    assert!(!ed.content_offscreen());
    draw(&mut ed, p(-1000.0, -1000.0), p(-900.0, -900.0));
    ed.handle(InputEvent::SelectTool(Tool::Rectangle));
    draw(&mut ed, p(2000.0, 2000.0), p(2100.0, 2100.0));
    assert!(
        ed.content_offscreen(),
        "viewport is in a gap inside the document bounds"
    );
    ed.handle(InputEvent::FitContent);
    assert!(!ed.content_offscreen());
    key(&mut ed, 'z', Modifiers { ctrl: true, ..NONE });
    ed.handle(InputEvent::Pan {
        by: ScreenVec::new(10000.0, 10000.0),
    });
    assert!(ed.content_offscreen());
    key(&mut ed, 'z', Modifiers { ctrl: true, ..NONE });
    assert!(
        !ed.content_offscreen(),
        "empty scene never offers return to content"
    );
}

#[test]
fn fit_all_and_selection_handle_wide_flat_paths_and_empty_scene() {
    let mut ed = Editor::default();
    let shift = Modifiers {
        shift: true,
        ..NONE
    };
    assert_eq!(key(&mut ed, '1', shift), Redraw::Nothing);
    ed.handle(InputEvent::SelectTool(Tool::Line));
    draw(&mut ed, p(100.0, 200.0), p(5100.0, 200.0));
    key(&mut ed, 'a', Modifiers { ctrl: true, ..NONE });
    key(&mut ed, '2', shift);
    let bounds = ed.selection_bounds().expect("bounds");
    let a = ed.camera.world_to_screen(bounds.min());
    let b = ed.camera.world_to_screen(bounds.max());
    assert!(a.x >= 63.0 && b.x <= 961.0, "{a:?} {b:?}");
    assert!(a.y > 100.0 && b.y < 600.0);
    assert!(!ed.content_offscreen());
    assert_eq!(
        ed.tool(),
        Tool::Select,
        "fit selection must not pick rectangle"
    );
}
