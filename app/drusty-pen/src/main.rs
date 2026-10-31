//! The drusty-pen window. The only crate that imports `rune`: it turns rune events into
//! `pen_editor::InputEvent`s and presents editor state as rune elements.

mod design;
mod overlay;
mod paint;
mod palette;
mod present;
mod scene;
mod toolbar;

use pen_core::WorldPt;
use pen_editor::{Camera, Editor, InputEvent, Modifiers, Redraw, ScreenPt, ScreenVec};
use pen_render::DrawList;
use rune::{
    AppSpec, Cursor, Cx, Drag, El, Key, KeyEvent, LiveText, MagnifyEvent, Mods, Phase, Shape, Tx,
    WheelEvent, canvas, el, text,
};

/// The window's opening size, in points.
const WINDOW: (f32, f32) = (1024.0, 720.0);

/// Ctrl+wheel zoom speed: the zoom factor per point of wheel travel is `e^ZOOM_PER_POINT`.
const ZOOM_PER_POINT: f32 = 0.006;

/// App state: the editor, plus everything rune needs to present it. Presentation state is
/// written only by [`State::input`], from the editor; it is never read back into the editor.
struct State {
    editor: Editor,
    camera: present::LiveCamera,
    scene: scene::Scene,
    /// The element being drawn, rebuilt on every pointer move of a drag.
    draft: Shape,
    /// Scratch for the draft, so a drag doesn't allocate a fresh list per move.
    draft_paint: DrawList,
    overlay: overlay::Overlay,
}

impl State {
    fn new(camera: Camera) -> Self {
        Self {
            editor: Editor::new(camera),
            camera: present::LiveCamera::new(&camera),
            scene: scene::Scene::default(),
            draft: Shape::new(),
            draft_paint: DrawList::new(),
            overlay: overlay::Overlay::default(),
        }
    }

    /// Feeds one event to the editor and presents what changed, rebuilding as little as
    /// possible. Returns what the editor reported.
    fn input(&mut self, event: InputEvent, tx: &mut Tx) -> Redraw {
        let camera = *self.editor.camera();
        let redraw = self.editor.handle(event);
        if *self.editor.camera() != camera {
            self.camera.show(self.editor.camera());
        }
        self.overlay.show(&self.editor);
        match redraw {
            // Only live values changed: no view rebuild, no layout.
            Redraw::Nothing | Redraw::Camera => tx.live_only(),
            Redraw::Draft | Redraw::Tool | Redraw::Scene => self.show_feedback(),
        }
        redraw
    }

    fn show_feedback(&mut self) {
        self.scene
            .show(self.editor.document(), self.editor.preview());
        self.draft_paint.clear();
        if let Some(draft) = self.editor.draft() {
            pen_render::draw(&draft.kind, &draft.props, &mut self.draft_paint);
        }
        self.draft = paint::shape(self.draft_paint.prims());
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("editor", &self.editor)
            .finish_non_exhaustive()
    }
}

fn mods(m: Mods) -> Modifiers {
    Modifiers {
        shift: m.shift,
        alt: m.alt,
        // Cmd on macOS, Ctrl elsewhere: the platform's command modifier.
        ctrl: m.cmd,
    }
}

fn init() -> State {
    let viewport = ScreenVec::new(WINDOW.0, WINDOW.1);
    let camera = present::camera_arg(rune::extra_args())
        .unwrap_or_else(|| Camera::centred_on(WorldPt::ORIGIN, viewport, 1.0));
    State::new(camera)
}

/// A zero-size layer at the world origin. The world node is laid out the same way, so
/// `live_scale` (which scales about the node's centre) scales about (0, 0).
fn layer(shape: Shape, key: &str) -> El {
    canvas(shape)
        .key(key)
        .absolute(0.0, 0.0)
        .size(0.0, 0.0)
        .transition(rune::Transition::none())
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let world = el()
        .key("world")
        .absolute(0.0, 0.0)
        .size(0.0, 0.0)
        .live_offset(&s.camera.offset)
        .live_scale(&s.camera.scale)
        // A drag rebuilds the view on every move; the committed scene is reused unless the
        // document changed.
        .child(s.scene.view())
        .child(layer(s.draft.clone(), "draft"));

    el().key("viewport")
        .w_full()
        .h_full()
        .fill(design::c(design::CANVAS))
        .focusable(true)
        .cursor(if s.editor.tool() == pen_editor::Tool::Select {
            Cursor::Default
        } else {
            Cursor::Crosshair
        })
        .child(world)
        .child(s.overlay.view())
        .child(toolbar::toolbar(s.editor.tool()))
        .child(zoom_pill(&s.camera.zoom_label))
        .on_drag(|s: &mut State, d: &mut Drag, tx| {
            let at = ScreenPt::new(d.x, d.y);
            let mods = mods(tx.mods());
            let event = match d.phase {
                Phase::Down => InputEvent::PointerDown { at, mods },
                Phase::Move => InputEvent::PointerMove { at, mods },
                Phase::Up if d.cancelled() => InputEvent::Cancel,
                Phase::Up => InputEvent::PointerUp { at, mods },
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

/// Keys no focused control took. Tool shortcuts, Escape, and modifier changes mid-drag.
fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let mods = mods(ev.mods);
    let event = match &ev.key {
        // Never consumed: a modifier is also part of whatever comes next.
        Key::Modifier { .. } => {
            s.input(InputEvent::Modifiers(mods), tx);
            return false;
        }
        Key::Command(c) => InputEvent::Key {
            key: pen_editor::Key::Char(c.to_ascii_lowercase()),
            mods,
        },
        Key::Escape
        | Key::Delete
        | Key::Backspace
        | Key::Left
        | Key::Right
        | Key::Up
        | Key::Down => InputEvent::Key {
            key: match ev.key {
                Key::Escape => pen_editor::Key::Escape,
                Key::Delete => pen_editor::Key::Delete,
                Key::Backspace => pen_editor::Key::Backspace,
                Key::Left => pen_editor::Key::Left,
                Key::Right => pen_editor::Key::Right,
                Key::Up => pen_editor::Key::Up,
                _ => pen_editor::Key::Down,
            },
            mods,
        },
        Key::Text(t) => {
            let mut chars = t.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                return false;
            };
            InputEvent::Key {
                key: pen_editor::Key::Char(c.to_ascii_lowercase()),
                mods,
            }
        }
        _ => return false,
    };
    s.input(event, tx) != Redraw::Nothing
}

fn zoom_pill(label: &LiveText) -> El {
    design::raised(el().key("zoom"), design::R_STATUS)
        .pin(Some(design::S4), None, None, Some(design::S4))
        .py(design::PADDING)
        .child(
            text("100%")
                .live_text(label)
                // Fixed width: a live label never relayouts, so it must fit "3000%".
                .size(64.0, 18.0)
                .align(rune::Align::Center)
                .font(design::T_LABEL)
                .color(design::c(design::INK)),
        )
}

fn spec() -> AppSpec<State> {
    AppSpec {
        on_key: Some(on_key),
        ..AppSpec::new("drusty-pen", WINDOW, init, view)
    }
}

fn main() {
    // Modifier keys on their own, so pressing Shift mid-drag squares the shape at once.
    rune::App::new(spec()).modifier_keys().run();
}

#[cfg(test)]
mod tests;
