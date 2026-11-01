//! A screen-covering repeating grid. Camera changes update only its live phase and scale.
//! Release M5 dark/grid script, 19.5 s / 1171 frames, scale 1: mean frame CPU 0.646 ms,
//! p95 1.245 ms. The retained node shares its shape through view rebuilds.
use crate::{design as d, layer};
use pen_core::{Settings, WorldPt, WorldRect};
use pen_editor::{Camera, GRID_SPACING};
use rune::{El, Live, Shape, el};

pub(crate) struct Grid {
    offset: Live<[f32; 2]>,
    scale: Live<f32>,
    node: El,
}
impl Grid {
    pub(crate) fn new(camera: &Camera) -> Self {
        let mut shape = Shape::new();
        // Covers 4K windows even at the minimum 10% zoom, independent of document size.
        let extent = 40960.0;
        for i in 0..=2048 {
            let at = i as f32 * GRID_SPACING;
            shape = shape
                .stroke(vec![[at, 0.0], [at, extent]], 0.5, d::c(d::BORDER))
                .stroke(vec![[0.0, at], [extent, at]], 0.5, d::c(d::BORDER));
        }
        let offset = Live::new([0.0; 2]);
        let scale = Live::new(camera.zoom());
        let node = layer(
            shape,
            WorldRect::from_corners(WorldPt::ORIGIN, WorldPt::new(extent, extent)),
            "grid",
        )
        .live_offset(&offset)
        .live_scale(&scale)
        .origin(0.0, 0.0)
        .pixel_snap(false)
        .inert();
        let grid = Self {
            offset,
            scale,
            node,
        };
        grid.show(camera);
        grid
    }
    pub(crate) fn show(&self, camera: &Camera) {
        let spacing = GRID_SPACING * camera.zoom();
        let scroll = camera.scroll();
        self.offset.set([
            (-scroll.x * camera.zoom()).rem_euclid(spacing) - spacing,
            (-scroll.y * camera.zoom()).rem_euclid(spacing) - spacing,
        ]);
        self.scale.set(camera.zoom());
    }
    pub(crate) fn view(&self, settings: Settings) -> El {
        if !settings.grid {
            return el().size(0.0, 0.0);
        }
        self.node
            .clone()
            .opacity(if settings.dark { 0.15 } else { 0.7 })
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid").finish_non_exhaustive()
    }
}
