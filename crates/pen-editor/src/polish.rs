//! Inspection, local destinations, and read-only editor modes.
use crate::{Editor, InputEvent, Key, Modifiers, Redraw, Selection, Tool};
use pen_core::{Command, Document, Kind, WorldPt, WorldRect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modes {
    pub zen: bool,
    pub view: bool,
    pub stats: bool,
    pub snap_objects: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryField {
    X,
    Y,
    Width,
    Height,
    Angle,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Polish {
    pub modes: Modes,
    pub trace: Option<crate::tracing::Trace>,
    pub laser: Vec<(WorldPt, f32)>,
    pub guides: Vec<(WorldPt, WorldPt)>,
    pub snapping: crate::snapping::Snapping,
}
impl Polish {
    pub fn refresh(&mut self, doc: &Document) {
        self.snapping.refresh(doc);
    }
}
impl Editor {
    #[must_use]
    pub fn modes(&self) -> Modes {
        self.polish.modes
    }
    #[must_use]
    pub fn guides(&self) -> &[(WorldPt, WorldPt)] {
        &self.polish.guides
    }
    #[must_use]
    pub fn laser(&self) -> &[(WorldPt, f32)] {
        &self.polish.laser
    }
    #[must_use]
    pub fn lasso(&self) -> &[WorldPt] {
        self.polish
            .trace
            .as_ref()
            .filter(|t| !t.laser)
            .map_or(&[], |t| &t.points)
    }
    pub(crate) fn polish_key(&mut self, key: Key, mods: Modifiers) -> Option<Redraw> {
        if mods.alt
            && !mods.ctrl
            && let Key::Char(c) = key
        {
            match c {
                'z' => self.polish.modes.zen = !self.polish.modes.zen,
                'r' => {
                    self.finish_text();
                    self.cancel();
                    self.polish.modes.view = !self.polish.modes.view;
                    if self.polish.modes.view {
                        self.tool = Tool::Hand;
                        self.selection.clear();
                    }
                }
                '/' => self.polish.modes.stats = !self.polish.modes.stats,
                's' => {
                    self.polish.modes.snap_objects = !self.polish.modes.snap_objects;
                    self.polish.guides.clear();
                }
                _ => return None,
            }
            return Some(Redraw::Tool);
        }
        None
    }
    pub(crate) fn polish_event(&mut self, event: InputEvent) -> Redraw {
        match event {
            InputEvent::SampleColor { color, background } => {
                let redraw = self.set_property(if background {
                    pen_core::Property::Background(Some(color))
                } else {
                    pen_core::Property::Stroke(color)
                });
                self.tool = Tool::Select;
                if redraw == Redraw::Nothing {
                    Redraw::Tool
                } else {
                    redraw
                }
            }
            InputEvent::Tick(dt) => {
                if !dt.is_finite() || dt <= 0.0 || self.polish.laser.is_empty() {
                    return Redraw::Nothing;
                }
                for (_, age) in &mut self.polish.laser {
                    *age += dt;
                }
                self.polish.laser.retain(|(_, age)| *age < 0.9);
                Redraw::Draft
            }
            InputEvent::SelectElement(id) => {
                if self.document.get(id).is_none() {
                    return Redraw::Nothing;
                }
                self.finish_text();
                self.cancel();
                self.tool = Tool::Select;
                self.selection = Selection::only(pen_core::bound::owner(&self.document, id));
                self.fit(true);
                Redraw::Tool
            }
            InputEvent::PlaceScene { scene, at } => {
                if scene.document().is_err() {
                    return Redraw::Nothing;
                }
                let Some(bounds) = scene
                    .elements
                    .iter()
                    .map(pen_core::Element::bounds)
                    .reduce(WorldRect::union)
                else {
                    return Redraw::Nothing;
                };
                let p = self.camera.screen_to_world(at);
                self.paste_with_files(
                    &scene.elements,
                    WorldPt::new(p.x - bounds.centre().x, p.y - bounds.centre().y),
                    &scene.files,
                )
            }
            InputEvent::RenameFrame(name) => self.patch_selected(|e| {
                if let Kind::Frame { name: old, .. } = &mut e.kind {
                    *old = name.trim().chars().take(120).collect();
                    if old.is_empty() {
                        *old = "Frame".into();
                    }
                }
            }),
            InputEvent::SetLink(link) => {
                let valid = match &link {
                    Some(pen_core::Link::Web(url)) => {
                        (url.starts_with("https://") || url.starts_with("http://"))
                            && !url.chars().any(|c| c.is_control() || c.is_whitespace())
                    }
                    Some(pen_core::Link::Element(id)) => self.document.get(*id).is_some(),
                    None => true,
                };
                if !valid {
                    return Redraw::Nothing;
                }
                self.patch_selected(|e| e.link.clone_from(&link))
            }
            InputEvent::Geometry { field, value } => {
                if !value.is_finite() || value.abs() > 10_000_000.0 || self.selection.len() != 1 {
                    return Redraw::Nothing;
                }
                self.patch_selected(|e| {
                    let b = e.kind.bounds();
                    let p = b.min();
                    let (x, y, w, h) = match field {
                        GeometryField::X => (value, p.y, b.width(), b.height()),
                        GeometryField::Y => (p.x, value, b.width(), b.height()),
                        GeometryField::Width => (
                            p.x,
                            p.y,
                            value.max(1.0),
                            if matches!(e.kind, Kind::Text { .. }) {
                                b.height() * value.max(1.0) / b.width().max(0.001)
                            } else {
                                b.height()
                            },
                        ),
                        GeometryField::Height => (p.x, p.y, b.width(), value.max(1.0)),
                        GeometryField::Angle => {
                            if !matches!(e.kind, Kind::Frame { .. }) {
                                e.angle = value.to_radians().rem_euclid(std::f32::consts::TAU);
                            }
                            return;
                        }
                    };
                    *e = e.with_bounds(WorldRect::from_corners(
                        WorldPt::new(x, y),
                        WorldPt::new(x + w, y + h),
                    ));
                })
            }
            _ => Redraw::Nothing,
        }
    }
    fn patch_selected(&mut self, mut edit: impl FnMut(&mut pen_core::Element)) -> Redraw {
        if self.gesture.is_some() || self.text_session.is_some() || self.selection_locked() {
            return Redraw::Nothing;
        }
        let commands: Vec<_> = self
            .selection
            .elements(&self.document)
            .filter_map(|before| {
                let mut after = before.clone();
                edit(&mut after);
                (after != *before).then(|| Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                })
            })
            .collect();
        if !commands.is_empty() && self.execute(Command::Group(commands), self.selection.clone()) {
            Redraw::Scene
        } else {
            Redraw::Nothing
        }
    }
}
pub(crate) fn view_event(e: &InputEvent) -> bool {
    match e {
        InputEvent::Pan { .. }
        | InputEvent::Zoom { .. }
        | InputEvent::Viewport(_)
        | InputEvent::FitContent
        | InputEvent::Space(_)
        | InputEvent::PanStart { .. }
        | InputEvent::PanMove { .. }
        | InputEvent::PanEnd
        | InputEvent::PointerDown { .. }
        | InputEvent::PointerMove { .. }
        | InputEvent::PointerUp { .. }
        | InputEvent::Cancel
        | InputEvent::Tick(_)
        | InputEvent::SelectElement(_) => true,
        InputEvent::Key { key, mods } => {
            (mods.alt
                && (matches!(key, Key::Char('r' | 'z' | '/' | 's'))
                    || (mods.shift && *key == Key::Char('d'))))
                || (!mods.alt
                    && ((mods.ctrl && matches!(key, Key::Char('0' | '+' | '=' | '-' | '_')))
                        || (mods.shift && matches!(key, Key::Char('1' | '2')))))
        }
        _ => false,
    }
}
