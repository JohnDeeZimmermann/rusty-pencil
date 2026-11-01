use crate::{
    DrawList, Prim,
    export::{self, Options},
};
use pen_core::{Srgba, WorldPt, WorldRect};
use std::collections::BTreeMap;
fn rect(x: f32, y: f32, w: f32, h: f32) -> WorldRect {
    WorldRect::from_corners(WorldPt::new(x, y), WorldPt::new(x + w, y + h))
}
#[test]
fn frame_clipping_crops_real_png_pixels_and_svg_geometry() {
    let mut inner = DrawList::new();
    inner.push(Prim::Fill {
        points: vec![
            WorldPt::new(-10.0, 10.0),
            WorldPt::new(60.0, 10.0),
            WorldPt::new(60.0, 30.0),
            WorldPt::new(-10.0, 30.0),
        ],
        color: Srgba::rgb(0xe55555),
    });
    let mut list = DrawList::new();
    list.push(Prim::Clip {
        bounds: rect(0.0, 0.0, 40.0, 40.0),
        prims: inner.prims().to_vec(),
    });
    let options = Options {
        background: None,
        ..Default::default()
    };
    let bytes = export::png(&list, &BTreeMap::new(), options).unwrap();
    let pix = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
    assert_eq!((pix.width(), pix.height()), (40, 40));
    assert_eq!(pix.pixel(0, 20).unwrap().alpha(), 255);
    assert_eq!(pix.pixel(20, 0).unwrap().alpha(), 0);
    let svg = export::svg(&list, &BTreeMap::new(), options).unwrap();
    assert!(svg.contains("clipPath"));
}
#[test]
fn empty_frame_exports_at_its_exact_size() {
    let mut list = DrawList::new();
    list.push(Prim::Clip {
        bounds: rect(0.0, 0.0, 120.0, 80.0),
        prims: vec![],
    });
    let bytes = export::png(&list, &BTreeMap::new(), Options::default()).unwrap();
    let pix = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
    assert_eq!((pix.width(), pix.height()), (120, 80));
}
#[test]
fn moving_and_resizing_frames_updates_child_clips_in_live_scene() {
    let mut ed = pen_editor::Editor::default();
    for (tool, a, b) in [
        (pen_editor::Tool::Rectangle, (20.0, 20.0), (140.0, 80.0)),
        (pen_editor::Tool::Frame, (0.0, 0.0), (100.0, 100.0)),
    ] {
        ed.handle(pen_editor::InputEvent::SelectTool(tool));
        ed.handle(pen_editor::InputEvent::PointerDown {
            at: pen_editor::ScreenPt::new(a.0, a.1),
            mods: Default::default(),
        });
        ed.handle(pen_editor::InputEvent::PointerUp {
            at: pen_editor::ScreenPt::new(b.0, b.1),
            mods: Default::default(),
        });
    }
    let mut scene = crate::Scene::default();
    scene.update(ed.document(), ed.preview());
    let clip = |scene: &crate::Scene| {
        scene
            .parts()
            .iter()
            .flat_map(crate::ScenePart::prims)
            .find_map(|p| {
                if let Prim::Clip { bounds, .. } = p {
                    Some(*bounds)
                } else {
                    None
                }
            })
            .unwrap()
    };
    assert_eq!(clip(&scene), rect(0.0, 0.0, 100.0, 100.0));
    ed.handle(pen_editor::InputEvent::PointerDown {
        at: pen_editor::ScreenPt::new(30.0, 0.0),
        mods: Default::default(),
    });
    ed.handle(pen_editor::InputEvent::PointerMove {
        at: pen_editor::ScreenPt::new(60.0, 20.0),
        mods: Default::default(),
    });
    scene.update(ed.document(), ed.preview());
    assert_eq!(clip(&scene), rect(30.0, 20.0, 100.0, 100.0));
    ed.handle(pen_editor::InputEvent::Cancel);
    scene.update(ed.document(), ed.preview());
    ed.handle(pen_editor::InputEvent::PointerDown {
        at: pen_editor::ScreenPt::new(105.0, 50.0),
        mods: Default::default(),
    });
    ed.handle(pen_editor::InputEvent::PointerMove {
        at: pen_editor::ScreenPt::new(65.0, 50.0),
        mods: Default::default(),
    });
    scene.update(ed.document(), ed.preview());
    assert_eq!(clip(&scene), rect(0.0, 0.0, 60.0, 100.0));
}
