use pen_core::WorldPt;

/// A point in the canvas viewport, in screen points from its top-left (y down).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScreenPt {
    pub x: f32,
    pub y: f32,
}

impl ScreenPt {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A displacement in screen points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScreenVec {
    pub dx: f32,
    pub dy: f32,
}

impl ScreenVec {
    #[must_use]
    pub const fn new(dx: f32, dy: f32) -> Self {
        Self { dx, dy }
    }
}

/// Maps world points to viewport screen points: `screen = (world - scroll) * zoom`.
///
/// Invariant: `zoom` is finite and within [`Camera::MIN_ZOOM`, `Camera::MAX_ZOOM`]; `scroll`
/// is finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The world point drawn at the viewport's top-left corner.
    scroll: WorldPt,
    zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            scroll: WorldPt::ORIGIN,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub const MIN_ZOOM: f32 = 0.1;
    pub const MAX_ZOOM: f32 = 30.0;

    /// A camera at `zoom` with `world` at the centre of a `viewport` (width, height in screen
    /// points). Non-finite input falls back to the default camera's values.
    #[must_use]
    pub fn centred_on(world: WorldPt, viewport: ScreenVec, zoom: f32) -> Self {
        let zoom = clamp_zoom(zoom, 1.0);
        let scroll = WorldPt::new(
            world.x - viewport.dx / 2.0 / zoom,
            world.y - viewport.dy / 2.0 / zoom,
        );
        let mut camera = Self {
            scroll: WorldPt::ORIGIN,
            zoom,
        };
        camera.set_scroll(scroll);
        camera
    }

    #[must_use]
    pub fn scroll(&self) -> WorldPt {
        self.scroll
    }

    #[must_use]
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    #[must_use]
    pub fn world_to_screen(&self, p: WorldPt) -> ScreenPt {
        ScreenPt::new(
            (p.x - self.scroll.x) * self.zoom,
            (p.y - self.scroll.y) * self.zoom,
        )
    }

    #[must_use]
    pub fn screen_to_world(&self, p: ScreenPt) -> WorldPt {
        WorldPt::new(
            p.x / self.zoom + self.scroll.x,
            p.y / self.zoom + self.scroll.y,
        )
    }

    /// Moves the content by `by` screen points. Non-finite deltas are ignored.
    pub fn pan_by(&mut self, by: ScreenVec) {
        let scroll = WorldPt::new(
            self.scroll.x - by.dx / self.zoom,
            self.scroll.y - by.dy / self.zoom,
        );
        self.set_scroll(scroll);
    }

    /// Multiplies the zoom by `factor`, clamped to the zoom range, keeping the world point under
    /// `anchor` fixed. Non-positive or non-finite factors are ignored.
    pub fn zoom_about(&mut self, anchor: ScreenPt, factor: f32) {
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }
        let fixed = self.screen_to_world(anchor);
        self.zoom = clamp_zoom(self.zoom * factor, self.zoom);
        self.set_scroll(WorldPt::new(
            fixed.x - anchor.x / self.zoom,
            fixed.y - anchor.y / self.zoom,
        ));
    }

    fn set_scroll(&mut self, scroll: WorldPt) {
        if scroll.x.is_finite() && scroll.y.is_finite() {
            self.scroll = scroll;
        }
    }
}

fn clamp_zoom(zoom: f32, fallback: f32) -> f32 {
    if zoom.is_finite() {
        zoom.clamp(Camera::MIN_ZOOM, Camera::MAX_ZOOM)
    } else {
        fallback
    }
}
