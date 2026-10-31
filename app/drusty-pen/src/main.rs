//! The drusty-pen window. The only crate that imports `rune`: it turns rune events into
//! `pen_editor::InputEvent`s and presents editor state as rune elements.

mod palette;
mod present;
mod scene;

use pen_core::WorldPt;
use pen_editor::{Camera, Editor, InputEvent, ScreenPt, ScreenVec};
use rune::{AppSpec, Color, Cx, El, LiveText, MagnifyEvent, Tx, WheelEvent, canvas, el, text};

/// The window's opening size, in points.
const WINDOW: (f32, f32) = (1024.0, 720.0);

/// App state: the editor plus the rune live values that present its camera. The live values
/// are written only by [`State::input`], from the editor; they are never read back.
struct State {
    editor: Editor,
    camera: present::LiveCamera,
}

impl State {
    fn new(camera: Camera) -> Self {
        Self {
            editor: Editor::new(camera),
            camera: present::LiveCamera::new(&camera),
        }
    }

    fn input(&mut self, event: InputEvent, tx: &mut Tx) {
        match self.editor.handle(event) {
            pen_editor::Redraw::Camera => {
                self.camera.show(self.editor.camera());
                // Only live values changed: no view rebuild, no layout.
                tx.live_only();
            }
        }
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("editor", &self.editor)
            .finish_non_exhaustive()
    }
}

fn init() -> State {
    let viewport = ScreenVec::new(WINDOW.0, WINDOW.1);
    let camera = present::camera_arg(rune::extra_args())
        .unwrap_or_else(|| Camera::centred_on(WorldPt::ORIGIN, viewport, 1.0));
    State::new(camera)
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    // The world: one canvas in world points, laid out as a zero-size node at the viewport's
    // origin so `live_scale` (which scales about the node's centre) scales about (0, 0).
    let world = canvas(scene::test_scene())
        .key("world")
        .absolute(0.0, 0.0)
        .size(0.0, 0.0)
        .live_offset(&s.camera.offset)
        .live_scale(&s.camera.scale);

    el().key("viewport")
        .w_full()
        .h_full()
        .fill(Color::WHITE)
        .child(world)
        .child(hint())
        .child(zoom_pill(&s.camera.zoom_label))
        .on_wheel(|s: &mut State, e: WheelEvent, tx| {
            s.input(
                InputEvent::Pan {
                    by: ScreenVec::new(e.dx, e.dy),
                },
                tx,
            );
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

fn hint() -> El {
    text("Scroll to pan  ·  Ctrl+scroll or pinch to zoom")
        .key("hint")
        .font(rune::tok::type_::BODY)
        .color(palette::srgb(0x86_86_8e))
        .pin(Some(16.0), Some(14.0), None, None)
}

fn zoom_pill(label: &LiveText) -> El {
    el().key("zoom")
        .pin(Some(16.0), None, None, Some(16.0))
        .px(12.0)
        .py(6.0)
        .radius(8.0)
        .fill(palette::srgb(0xec_ec_f4))
        .child(
            text("100%")
                .live_text(label)
                .size(48.0, 18.0)
                .font(rune::tok::type_::LABEL)
                .color(palette::srgb(0x33_33_3a)),
        )
}

fn main() {
    rune::run(AppSpec::new("drusty-pen", WINDOW, init, view));
}

#[cfg(test)]
mod tests;
