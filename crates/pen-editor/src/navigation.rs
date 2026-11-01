//! View navigation and a bounds hierarchy refreshed only after document commands.
use crate::{Camera, Editor, InputEvent, Key, Modifiers, Redraw, ScreenPt, ScreenVec};
use pen_core::{Document, WorldRect};

#[derive(Debug, Clone)]
struct Node {
    bounds: WorldRect,
    children: Option<(Box<Node>, Box<Node>)>,
}

impl Node {
    fn build(rects: &mut [WorldRect]) -> Option<Self> {
        let bounds = rects.iter().copied().reduce(WorldRect::union)?;
        let children = if rects.len() > 1 {
            let horizontal = bounds.width() >= bounds.height();
            rects.sort_unstable_by(|a, b| {
                let a = a.centre();
                let b = b.centre();
                if horizontal {
                    a.x.total_cmp(&b.x)
                } else {
                    a.y.total_cmp(&b.y)
                }
            });
            let mid = rects.len() / 2;
            let (a, b) = rects.split_at_mut(mid);
            Node::build(a)
                .zip(Node::build(b))
                .map(|(a, b)| (Box::new(a), Box::new(b)))
        } else {
            None
        };
        Some(Self { bounds, children })
    }

    fn visible(&self, viewport: WorldRect) -> bool {
        let a = self.bounds;
        if a.max().x < viewport.min().x
            || a.min().x > viewport.max().x
            || a.max().y < viewport.min().y
            || a.min().y > viewport.max().y
        {
            return false;
        }
        match &self.children {
            Some((a, b)) => a.visible(viewport) || b.visible(viewport),
            None => true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Navigation {
    pub(crate) viewport: ScreenVec,
    pub(crate) space: bool,
    pub(crate) pan: Option<ScreenPt>,
    root: Option<Node>,
    revision: Option<u64>,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            viewport: ScreenVec::new(1024.0, 720.0),
            space: false,
            pan: None,
            root: None,
            revision: None,
        }
    }
}

impl Navigation {
    pub(crate) fn refresh(&mut self, doc: &Document) {
        if self.revision == Some(doc.revision()) {
            return;
        }
        let mut rects: Vec<_> = doc
            .elements()
            .iter()
            .map(|e| {
                let r = e.bounds();
                let pad = (e.props.stroke_width * 4.0).max(1.0);
                WorldRect::from_centre(r.centre(), r.width() / 2.0 + pad, r.height() / 2.0 + pad)
            })
            .collect();
        self.root = Node::build(&mut rects);
        self.revision = Some(doc.revision());
    }
}

impl Editor {
    #[must_use]
    pub fn viewport(&self) -> ScreenVec {
        self.navigation.viewport
    }

    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.navigation.pan.is_some()
    }
    #[must_use]
    pub fn space_held(&self) -> bool {
        self.navigation.space
    }
    #[must_use]
    pub fn content_offscreen(&self) -> bool {
        let Some(root) = &self.navigation.root else {
            return false;
        };
        let end = ScreenPt::new(self.navigation.viewport.dx, self.navigation.viewport.dy);
        !root.visible(WorldRect::from_corners(
            self.camera.screen_to_world(ScreenPt::default()),
            self.camera.screen_to_world(end),
        ))
    }

    pub(crate) fn pan_to(&mut self, at: ScreenPt) -> Redraw {
        if !at.x.is_finite() || !at.y.is_finite() {
            return Redraw::Nothing;
        }
        let Some(last) = self.navigation.pan else {
            return Redraw::Nothing;
        };
        self.navigation.pan = Some(at);
        self.camera
            .pan_by(ScreenVec::new(at.x - last.x, at.y - last.y));
        self.camera_moved()
    }

    pub(crate) fn fit(&mut self, selection: bool) -> Redraw {
        let bounds = if selection {
            self.selection.bounds(&self.document)
        } else {
            self.navigation.root.as_ref().map(|n| n.bounds)
        };
        let Some(bounds) = bounds else {
            return Redraw::Nothing;
        };
        let viewport = self.navigation.viewport;
        let zoom = ((viewport.dx - 128.0).max(1.0) / bounds.width().max(1.0))
            .min((viewport.dy - 192.0).max(1.0) / bounds.height().max(1.0))
            .min(1.0);
        self.camera = Camera::centred_on(bounds.centre(), viewport, zoom);
        self.camera_moved()
    }

    pub(crate) fn view_key(&mut self, key: Key, mods: Modifiers) -> Option<Redraw> {
        if mods.alt {
            return None;
        }
        if mods.shift && !mods.ctrl {
            return match key {
                Key::Char('1') => Some(self.fit(false)),
                Key::Char('2') => Some(self.fit(true)),
                _ => None,
            };
        }
        if !mods.ctrl {
            return None;
        }
        let factor = match key {
            Key::Char('+') | Key::Char('=') => 1.2,
            Key::Char('-') | Key::Char('_') => 1.0 / 1.2,
            Key::Char('0') => 1.0 / self.camera.zoom(),
            _ => return None,
        };
        let centre = ScreenPt::new(
            self.navigation.viewport.dx / 2.0,
            self.navigation.viewport.dy / 2.0,
        );
        Some(self.handle(InputEvent::Zoom {
            about: centre,
            factor,
        }))
    }
}
