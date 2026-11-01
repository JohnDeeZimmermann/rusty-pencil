use crate::{Handle, Modifiers};
use pen_core::{WorldPt, WorldRect};

/// Resize from the original geometry on every update, so modifier changes never accumulate
/// rounding error. Crossing the anchor flips the box; a one-screen-point minimum avoids
/// degenerate geometry. Side handles with Shift expand the other axis about its centre.
pub(crate) fn resize(
    bounds: WorldRect,
    handle: Handle,
    delta: WorldPt,
    mods: Modifiers,
    zoom: f32,
) -> WorldRect {
    let (hx, hy) = handle.axes();
    let centre = bounds.centre();
    let min = 1.0 / zoom;
    let multiplier = if mods.alt { 2.0 } else { 1.0 };
    let raw_w = bounds.width() + hx * delta.x * multiplier;
    let raw_h = bounds.height() + hy * delta.y * multiplier;
    let mut w = raw_w.abs().max(min);
    let mut h = raw_h.abs().max(min);
    if mods.shift {
        let ratio = bounds.width().max(min) / bounds.height().max(min);
        if hx == 0.0 {
            w = h * ratio;
        } else if hy == 0.0 || w / h > ratio {
            h = w / ratio;
        } else {
            w = h * ratio;
        }
        // Keep both dimensions nonzero even for very thin shapes.
        let scale = (min / w).max(min / h).max(1.0);
        w *= scale;
        h *= scale;
    }
    if mods.alt {
        return WorldRect::from_centre(centre, w / 2.0, h / 2.0);
    }
    let x = if hx == 0.0 {
        centre.x - w / 2.0
    } else {
        let anchor = centre.x - hx * bounds.width() / 2.0;
        anchor.min(anchor + hx * raw_w.signum() * w)
    };
    let y = if hy == 0.0 {
        centre.y - h / 2.0
    } else {
        let anchor = centre.y - hy * bounds.height() / 2.0;
        anchor.min(anchor + hy * raw_h.signum() * h)
    };
    WorldRect::from_corners(WorldPt::new(x, y), WorldPt::new(x + w, y + h))
}
