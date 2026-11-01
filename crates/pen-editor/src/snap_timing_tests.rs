//! Reproducible release timings for the per-move path, including live membership checks.
use super::*;
use pen_core::{WorldPt, WorldRect};
#[test]
#[ignore = "release timing harness: cargo test --release -p pen-editor snapping_timing -- --ignored --nocapture"]
fn snapping_timing() {
    for count in [1_000, 10_000] {
        let mut ed = Editor::default();
        let mut commands = Vec::new();
        for i in 0..count {
            let id = ed.document.alloc_id();
            let x = (i % 100) as f32 * 80.0;
            let y = (i / 100) as f32 * 80.0;
            commands.push(Command::Add {
                element: Element::new(
                    id,
                    Kind::Rect(WorldRect::from_corners(
                        WorldPt::new(x, y),
                        WorldPt::new(x + 40.0, y + 40.0),
                    )),
                    Props::default(),
                ),
                at: i,
            });
        }
        ed.document.apply(Command::Group(commands)).unwrap();
        ed.selection = Selection::only(ed.document.elements()[0].id());
        ed.handle(InputEvent::SelectTool(Tool::Select));
        ed.handle(InputEvent::Key {
            key: Key::Char('s'),
            mods: Modifiers {
                alt: true,
                ..Default::default()
            },
        });
        ed.handle(InputEvent::PointerDown {
            at: ScreenPt::new(20.0, 20.0),
            mods: Modifiers::NONE,
        });
        ed.handle(InputEvent::PointerMove {
            at: ScreenPt::new(30.0, 30.0),
            mods: Modifiers::NONE,
        });
        let revision = ed.document.revision();
        let mut samples = Vec::new();
        for i in 0..4000 {
            let p = (i % 400) as f32;
            let start = std::time::Instant::now();
            ed.handle(InputEvent::PointerMove {
                at: ScreenPt::new(30.0 + p * 0.25, 30.0 + p * 0.5),
                mods: Modifiers::NONE,
            });
            std::hint::black_box(ed.preview());
            samples.push(start.elapsed().as_secs_f64() * 1_000_000.0);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "{count} elements: mean {:.3} µs, p95 {:.3} µs",
            samples.iter().sum::<f64>() / samples.len() as f64,
            samples[samples.len() * 95 / 100]
        );
        assert_eq!(ed.document.revision(), revision);
    }
}
