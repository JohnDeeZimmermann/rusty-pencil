//! Real input and frame timings for a connected multiselection like the reported scene.
use super::tests::{FRAME, app, drag, key};
use super::*;
use rune::Runtime;

fn connected_selection() -> Runtime<State> {
    let mut rt = app();
    key(&mut rt, "q");
    for (a, b) in [
        ((310.0, 300.0), (390.0, 380.0)),
        ((470.0, 320.0), (590.0, 460.0)),
        ((760.0, 330.0), (940.0, 530.0)),
        ((470.0, 130.0), (680.0, 250.0)),
    ] {
        key(&mut rt, "r");
        drag(&mut rt, a, b);
    }
    key(&mut rt, "v");
    drag(&mut rt, (575.0, 190.0), (575.0, 190.0));
    drag(&mut rt, (575.0, 190.0), (575.0, 190.0));
    key(&mut rt, "Das ist ein Text");
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    for (a, b) in [
        ((398.0, 340.0), (462.0, 390.0)),
        ((598.0, 390.0), (752.0, 430.0)),
    ] {
        key(&mut rt, "a");
        drag(&mut rt, a, b);
    }
    key(&mut rt, "v");
    rt.key(KeyEvent {
        key: Key::Command('a'),
        mods: Mods {
            ctrl: true,
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.document().elements().len(), 7);
    assert_eq!(rt.state.editor.selection().len(), 6);
    rt
}

#[test]
#[ignore = "release timing harness: cargo test --release -p rusty-pencil multiselect_drag_timing -- --ignored --nocapture"]
fn multiselect_drag_timing() {
    let mut rt = connected_selection();
    rt.pointer_move(650.0, 300.0);
    rt.pointer_down(650.0, 300.0);
    rt.frame(FRAME);
    let mut times = Vec::new();
    for frame in 0..360 {
        let start = std::time::Instant::now();
        rt.pointer_move(
            650.0 + (frame as f32 * 0.04).sin() * 60.0,
            300.0 + frame as f32 * 0.15,
        );
        rt.frame(FRAME);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "multiselect input+frame CPU: mean {:.3} ms, p50 {:.3} ms, p95 {:.3} ms",
        times.iter().sum::<f64>() / times.len() as f64,
        times[times.len() / 2],
        times[times.len() * 95 / 100]
    );
}

#[test]
fn connected_multiselect_moves_live_and_commits_as_one_undo() {
    let mut rt = connected_selection();
    let before = rt.state.editor.document().elements().to_vec();
    let revision = rt.state.editor.document().revision();
    rt.pointer_move(650.0, 300.0);
    rt.pointer_down(650.0, 300.0);
    rt.pointer_move(660.0, 310.0);
    rt.frame(FRAME);
    let commits = rt.stats.commits;
    // Include subpixel motion: neither text nor arrows may force layout or a view commit.
    for i in 1..=30 {
        rt.pointer_move(660.0 + i as f32 * 0.25, 310.0 + i as f32 * 0.5);
        rt.frame(FRAME);
        assert_eq!(rt.stats.commits, commits, "move {i} must be live-only");
        assert_eq!(rt.state.editor.document().revision(), revision);
        assert_eq!(rt.state.editor.preview().len(), 7);
    }
    rt.pointer_up(667.5, 325.0);
    rt.frame(FRAME);
    for (a, b) in before.iter().zip(rt.state.editor.document().elements()) {
        let a = a.kind.bounds();
        let b = b.kind.bounds();
        assert!((b.min().x - a.min().x - 17.5).abs() < 0.001);
        assert!((b.min().y - a.min().y - 25.0).abs() < 0.001);
    }
    rt.key(KeyEvent {
        key: Key::Command('z'),
        mods: Mods {
            ctrl: true,
            cmd: true,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
    for (a, b) in before.iter().zip(rt.state.editor.document().elements()) {
        assert_eq!(a.kind, b.kind);
    }
}

#[test]
fn live_move_presents_the_offset_and_escape_restores_the_scene() {
    let mut rt = connected_selection();
    rt.pointer_move(650.0, 300.0);
    rt.pointer_down(650.0, 300.0);
    rt.pointer_move(660.0, 310.0);
    rt.frame(FRAME);
    let rects = |rt: &Runtime<State>| -> Vec<[f32; 4]> {
        rt.elements()
            .iter()
            .filter(|e| {
                e.key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("scene-part-"))
            })
            .map(|e| e.rect)
            .collect()
    };
    let before = rects(&rt);
    let commits = rt.stats.commits;
    rt.pointer_move(660.25, 310.5);
    rt.frame(FRAME);
    assert_eq!(rt.stats.commits, commits);
    let after = rects(&rt);
    assert!(
        before.iter().zip(&after).any(|(a, b)| {
            (b[0] - a[0] - 0.25).abs() < 0.001 && (b[1] - a[1] - 0.5).abs() < 0.001
        }),
        "live offsets must reach Rune on the same frame"
    );
    rt.key(KeyEvent {
        key: Key::Escape,
        mods: Mods::default(),
    });
    rt.frame(FRAME);
    assert!(rt.state.editor.preview().is_empty());
    assert_eq!(rects(&rt).len(), 1, "cancel restores the static scene");
}
