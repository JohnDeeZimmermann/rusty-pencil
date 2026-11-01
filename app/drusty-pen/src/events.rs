//! Canvas events remain GUI adapters: navigation and drawing state live in the editor.
use crate::{State, mods};
use pen_editor::{InputEvent, ScreenPt, ScreenVec};
use rune::{Drag, El, Key, MagnifyEvent, Phase, PointerButton, WheelEvent};

const ZOOM_PER_POINT: f32 = 0.006;

pub(crate) fn canvas_input(node: El) -> El {
    node.on_drop_files(|s: &mut State, paths, tx| {
        if let Some(path) = paths.first() {
            s.open_path(path, tx);
        }
    })
    .pointer_buttons([PointerButton::Middle])
    .on_layout_tx(|s: &mut State, [_, _, w, h], tx| {
        s.input(InputEvent::Viewport(ScreenVec::new(w, h)), tx);
    })
    .on_window_focus(|s: &mut State, focused, tx| {
        if !focused {
            s.sync_text(tx);
            s.input(InputEvent::FinishText, tx);
            s.input(InputEvent::Cancel, tx);
        }
    })
    .on_key_up(|s: &mut State, e, tx| {
        if matches!(&e.key, Key::Text(t) if t == " ") {
            s.input(InputEvent::Space(false), tx);
            true
        } else {
            false
        }
    })
    .on_pointer_move_local(|s: &mut State, p, tx| {
        s.input(
            InputEvent::PointerMove {
                at: ScreenPt::new(p.x, p.y),
                mods: mods(tx.mods()),
            },
            tx,
        );
    })
    .on_drag(|s: &mut State, d: &mut Drag, tx| {
        let at = ScreenPt::new(d.x, d.y);
        let mods = mods(tx.mods());
        let event = if tx.button() == PointerButton::Middle {
            match d.phase {
                Phase::Down => InputEvent::PanStart { at },
                Phase::Move => InputEvent::PanMove { at },
                Phase::Up => InputEvent::PanEnd,
            }
        } else {
            match d.phase {
                Phase::Down if d.clicks == 2 => InputEvent::DoubleClick { at, mods },
                Phase::Down => InputEvent::PointerDown { at, mods },
                Phase::Move => InputEvent::PointerMove { at, mods },
                Phase::Up if d.cancelled() => InputEvent::Cancel,
                Phase::Up => InputEvent::PointerUp { at, mods },
            }
        };
        s.input(event, tx);
    })
    .on_wheel(|s: &mut State, e: WheelEvent, tx| {
        let event = if e.mods.cmd || e.mods.ctrl {
            InputEvent::Zoom {
                about: ScreenPt::new(e.x, e.y),
                factor: (e.dy * ZOOM_PER_POINT).exp(),
            }
        } else {
            InputEvent::Pan {
                by: ScreenVec::new(e.dx, e.dy),
            }
        };
        s.input(event, tx);
        true
    })
    .on_magnify(|s: &mut State, e: MagnifyEvent, tx| {
        s.input(
            InputEvent::Zoom {
                about: ScreenPt::new(e.x, e.y),
                factor: 1.0 + e.delta,
            },
            tx,
        );
        true
    })
}
