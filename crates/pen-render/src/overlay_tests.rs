use super::*;
use pen_core::{Command, Document, Element, WorldRect};
use pen_editor::{Editor, InputEvent, Key, Modifiers, ScreenPt, Tool};

#[test]
fn cache_rebuilds_only_the_patch_on_change_undo_and_redo() {
    let mut doc = Document::new();
    let mut cache = SceneCache::new();
    for i in 0..3 {
        let element = Element::new(
            doc.alloc_id(),
            Kind::Rect(WorldRect::from_centre(
                WorldPt::new(i as f32 * 100.0, 0.0),
                20.0,
                20.0,
            )),
            Props::default(),
        );
        doc.apply(Command::Add { element, at: i }).expect("add");
    }
    assert_eq!(cache.update(&doc), 3);
    let before = doc.elements()[1].clone();
    let after = before.with_bounds(before.kind.bounds().translated(10.0, 20.0));
    let undo = doc
        .apply(Command::Patch {
            id: before.id(),
            before,
            after,
        })
        .expect("patch");
    assert_eq!(cache.update(&doc), 1);
    let redo = doc.apply(undo).expect("undo");
    assert_eq!(cache.update(&doc), 1);
    doc.apply(redo).expect("redo");
    assert_eq!(cache.update(&doc), 1);
    assert_eq!(cache.update(&doc), 0);
}

#[test]
fn handles_remain_eight_screen_points_at_every_zoom() {
    let mut ed = Editor::default();
    let mods = Modifiers::NONE;
    ed.handle(InputEvent::PointerDown {
        at: ScreenPt::new(100.0, 100.0),
        mods,
    });
    ed.handle(InputEvent::PointerUp {
        at: ScreenPt::new(300.0, 200.0),
        mods,
    });
    ed.handle(InputEvent::SelectTool(Tool::Select));
    ed.handle(InputEvent::Key {
        key: Key::Char('a'),
        mods: Modifiers { ctrl: true, ..mods },
    });
    let mut list = Vec::new();
    for factor in [1.0, 0.1, 20.0, 15.0] {
        ed.handle(InputEvent::Zoom {
            about: ScreenPt::new(150.0, 150.0),
            factor,
        });
        overlay(&ed, &mut list);
        assert_eq!(list.len(), 9);
        for handle in &list[1..] {
            assert!((handle.rect[2] - 8.0).abs() < f32::EPSILON);
            assert!((handle.rect[3] - 8.0).abs() < f32::EPSILON);
        }
        let bounds = ed.selection_bounds().expect("selected");
        assert!((list[0].rect[2] - (bounds.width() * ed.camera().zoom() + 10.0)).abs() < 1e-3);
    }
}
