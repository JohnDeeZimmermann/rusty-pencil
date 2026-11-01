//! The drusty-pen window. The only crate that imports `rune`: it turns rune events into
//! `pen_editor::InputEvent`s and presents editor state as rune elements.

mod design;
mod events;
mod files;
mod grid;
mod navigation;
mod overlay;
mod paint;
mod palette;
mod present;
mod properties;
mod property_icons;
mod scene;
mod text;
mod toolbar;

use pen_core::{WorldPt, WorldRect};
use pen_editor::{Camera, Editor, InputEvent, Modifiers, Redraw, ScreenVec};
use pen_render::DrawList;
use rune::{AppSpec, Cursor, Cx, El, Key, KeyEvent, Live, Mods, Shape, Tx, canvas, el};

/// The window's opening size, in points.
const WINDOW: (f32, f32) = (1024.0, 720.0);

/// App state: the editor, plus everything rune needs to present it. Presentation state is
/// written only by [`State::input`], from the editor; it is never read back into the editor.
struct State {
    editor: Editor,
    text_edit: rune::TextEdit,
    hex: [rune::TextEdit; 3],
    fonts: rune::text::TextSystem,
    files: files::Files,
    camera: present::LiveCamera,
    grid: grid::Grid,
    scene: scene::Scene,
    /// The element being drawn, rebuilt on every pointer move of a drag.
    draft: Shape,
    draft_bounds: WorldRect,
    /// Scratch for the draft, so a drag doesn't allocate a fresh list per move.
    draft_paint: DrawList,
    overlay: overlay::Overlay,
    content_button: Live<f32>,
}

impl State {
    fn new(camera: Camera) -> Self {
        Self {
            editor: Editor::new(camera),
            text_edit: rune::TextEdit::default(),
            hex: [
                rune::TextEdit::new("#1e1e1e"),
                rune::TextEdit::default(),
                rune::TextEdit::new("#f8f9f7"),
            ],
            fonts: text::system(),
            files: files::Files::default(),
            camera: present::LiveCamera::new(&camera),
            grid: grid::Grid::new(&camera),
            scene: scene::Scene::default(),
            draft: Shape::new(),
            draft_bounds: WorldRect::default(),
            draft_paint: DrawList::new(),
            overlay: overlay::Overlay::default(),
            content_button: Live::new(0.0),
        }
    }

    /// Feeds one event to the editor and presents what changed, rebuilding as little as
    /// possible. Returns what the editor reported.
    fn input(&mut self, event: InputEvent, tx: &mut Tx) -> Redraw {
        let event = if let InputEvent::Key {
            key: pen_editor::Key::Char(c),
            mods,
        } = &event
            && mods.ctrl
            && mods.shift
            && matches!(c, '<' | '>' | ',' | '.')
        {
            let (font, size, align) = self.editor.text_style();
            let sizes = [16.0, 20.0, 28.0, 36.0];
            let size = if matches!(c, '>' | '.') {
                sizes.into_iter().find(|s| *s > size).unwrap_or(36.0)
            } else {
                sizes.into_iter().rev().find(|s| *s < size).unwrap_or(16.0)
            };
            InputEvent::TextStyle { font, size, align }
        } else {
            event
        };
        // Palette fields follow edits and membership changes, never camera or draft moves.
        let refresh_properties = matches!(
            &event,
            InputEvent::Property(pen_core::Property::Stroke(_) | pen_core::Property::Background(_))
                | InputEvent::PointerDown { .. }
                | InputEvent::PointerUp { .. }
                | InputEvent::DoubleClick { .. }
                | InputEvent::SelectTool(_)
                | InputEvent::Key { .. }
                | InputEvent::FinishText
                | InputEvent::Settings(_)
                | InputEvent::Cancel
                | InputEvent::EndProperty { .. }
        );
        let camera = *self.editor.camera();
        let revision = self.editor.document().revision();
        let settings = self.editor.settings();
        let style = self.editor.style();
        let text_defaults = self.editor.text_defaults();
        let text_changed = matches!(&event, InputEvent::TextChanged { content, .. } if self.editor.text_session().is_some_and(|s| matches!(&s.element.kind, pen_core::Kind::Text { content: old, .. } if content != old)));
        let old_text = self.editor.text_session().map(|s| s.element.id());
        let redraw = if let InputEvent::TextStyle { font, size, align } = event {
            let fonts = &mut self.fonts;
            self.editor
                .style_text(font, size, align, |content, font, size, wrap| {
                    text::measure(fonts, content, font, size, wrap)
                })
        } else {
            self.editor.handle(event)
        };
        self.text_session_changed(old_text);
        if refresh_properties {
            properties::sync(self);
        }
        if self.editor.document().revision() != revision
            || *self.editor.camera() != camera
            || self.editor.style() != style
            || self.editor.text_defaults() != text_defaults
            || self.editor.settings() != settings
            || text_changed
        {
            self.files.changed(tx);
        }
        if *self.editor.camera() != camera {
            self.camera.show(self.editor.camera());
            self.grid.show(self.editor.camera());
        }
        self.overlay.show(&self.editor);
        self.content_button.set(if self.editor.content_offscreen() {
            1.0
        } else {
            0.0
        });
        match redraw {
            // Only live values changed: no view rebuild, no layout.
            Redraw::Nothing | Redraw::Camera => tx.live_only(),
            Redraw::Draft | Redraw::Tool | Redraw::Scene => self.show_feedback(),
        }
        redraw
    }

    fn show_feedback(&mut self) {
        self.scene.show_with_settings(
            self.editor.document(),
            self.editor.preview(),
            self.editor.settings(),
        );
        self.draft_paint.clear();
        if let Some(draft) = self.editor.draft() {
            pen_render::draw_seeded(&draft.kind, &draft.props, draft.seed, &mut self.draft_paint);
        }
        let themed;
        let prims = if self.editor.settings().dark {
            themed = paint::themed(self.draft_paint.prims());
            &themed
        } else {
            self.draft_paint.prims()
        };
        (self.draft, self.draft_bounds) = paint::bounded(prims);
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
    let mut state = State::new(camera);
    state.restore();
    state
}

/// Bounded paint under the world transform; Rune uses the rect to cull offscreen canvases.
fn layer(shape: Shape, bounds: WorldRect, key: &str) -> El {
    canvas(shape)
        .key(key)
        .absolute(bounds.min().x, bounds.min().y)
        .size(bounds.width(), bounds.height())
        .transition(rune::Transition::none())
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let world = el()
        .key("world")
        .instant_changes()
        .absolute(0.0, 0.0)
        .size(0.0, 0.0)
        .live_offset(&s.camera.offset)
        .live_scale(&s.camera.scale)
        // Rune snaps a scaled layer to device pixels at rest, lets go of that correction as
        // soon as it moves and eases it back once still, so shapes hop up-left on the first
        // pan or zoom frame and drift back after. Free-form drawings don't need hard pixel
        // edges; draw them at their exact sub-pixel positions on every frame instead.
        .pixel_snap(false)
        // A drag rebuilds the view on every move; the committed scene is reused unless the
        // document changed.
        .child(s.scene.view())
        .child(layer(s.draft.clone(), s.draft_bounds, "draft"))
        .child(text::editor(s));

    events::canvas_input(
        el().key("viewport")
            .w_full()
            .h_full()
            .fill(design::themed(
                s.editor.settings().canvas,
                s.editor.settings().dark,
            ))
            .focusable(s.editor.tool() != pen_editor::Tool::Hand && !s.editor.space_held())
            .preserve_focus_on_press(true)
            .cursor(
                if s.editor.tool() == pen_editor::Tool::Hand || s.editor.space_held() {
                    Cursor::Grab
                } else if s.editor.tool() == pen_editor::Tool::Select {
                    Cursor::Default
                } else {
                    Cursor::Crosshair
                },
            )
            .child(s.grid.view(s.editor.settings()))
            .child(world)
            .child(s.overlay.view(s.editor.settings().dark))
            .child(toolbar::toolbar(&s.editor))
            .child(navigation::zoom_pill(
                &s.camera.zoom_label,
                s.editor.settings().dark,
            ))
            .child(navigation::content_button(
                &s.content_button,
                s.editor.settings().dark,
            ))
            .child(properties::panel(s))
            .child(files::controls(s))
            .child(files::confirmation(s)),
    )
}

/// Keys no focused control took. Tool shortcuts, Escape, and modifier changes mid-drag.
fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if files::key(s, ev, tx) {
        return true;
    }
    let mods = mods(ev.mods);
    if matches!(&ev.key, Key::Text(t) if t == " ") && !mods.ctrl && !mods.alt {
        s.input(InputEvent::Space(true), tx);
        return true;
    }
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
        | Key::Enter
        | Key::Delete
        | Key::Backspace
        | Key::Left
        | Key::Right
        | Key::Up
        | Key::Down => InputEvent::Key {
            key: match ev.key {
                Key::Escape => pen_editor::Key::Escape,
                Key::Enter => pen_editor::Key::Enter,
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

fn spec() -> AppSpec<State> {
    AppSpec {
        on_key: Some(on_key),
        fonts: text::FONTS,
        families: text::FAMILIES,
        poll: Some(files::poll),
        ..AppSpec::new("drusty-pen", WINDOW, init, view)
    }
}

fn main() {
    // Modifier keys on their own, so pressing Shift mid-drag squares the shape at once.
    rune::App::new(spec())
        .on_close(files::close)
        .modifier_keys()
        .run();
}

#[cfg(test)]
mod pan_snap_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod m5_tests;
