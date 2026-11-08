//! Smooth seeded perturbations of our canonical contours. No camera input is required:
//! flattening at the maximum zoom keeps cached geometry stable throughout pan and zoom.
use crate::{DrawList, Prim};
use pen_core::{Element, Kind, Roughness, WorldPt};
use std::f32::consts::TAU;

pub fn draw_element(element: &Element, out: &mut DrawList) {
    if element.props.opacity == 0 {
        return;
    }
    let first = out.prims.len();
    draw_seeded(&element.kind, &element.props, element.seed(), out);
    if element.angle == 0.0 {
        return;
    }
    let centre = element.kind.bounds().centre();
    for prim in &mut out.prims[first..] {
        match prim {
            Prim::Clip { .. } => {}
            Prim::Image { angle, .. } | Prim::Text { angle, .. } => *angle = element.angle,
            Prim::Fill { points, .. } | Prim::Stroke { points, .. } => {
                for point in points {
                    *point = pen_core::rotate(*point, centre, element.angle);
                }
            }
        }
    }
}

pub fn draw_seeded(kind: &Kind, props: &pen_core::Props, seed: u32, out: &mut DrawList) {
    crate::draw_with(kind, props, Wobble::new(kind, props, seed), out);
}

/// One element's seeded wobble. Each stroke perturbs with the seed offset by its index in the
/// element's paint, so neighbouring strokes (outline, hatch lines, heads) wobble differently.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Wobble {
    seed: u32,
    amount: f32,
}

impl Wobble {
    fn new(kind: &Kind, props: &pen_core::Props, seed: u32) -> Option<Self> {
        if matches!(
            kind,
            Kind::Frame { .. } | Kind::Text { .. } | Kind::Freedraw(_)
        ) {
            return None;
        }
        let amount = match props.roughness {
            Roughness::Architect => return None,
            Roughness::Artist => 0.7,
            Roughness::Cartoonist => 1.6,
        };
        Some(Self { seed, amount })
    }

    /// Resamples a whole contour. Apply it before cutting dashes: the sample spacing scales
    /// with contour length, so wobbling each dash on its own would turn an 8 pt dash into
    /// hundreds of points (and a dot into ~100), which Rune draws as one instance each.
    pub(crate) fn apply(self, points: &[WorldPt], closed: bool, index: usize) -> Vec<WorldPt> {
        perturb(
            points,
            closed,
            self.seed.wrapping_add(index as u32),
            self.amount,
        )
    }
}

fn perturb(points: &[WorldPt], closed: bool, seed: u32, amount: f32) -> Vec<WorldPt> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let count = points.len() - usize::from(!closed);
    let length = (0..count)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            (b.x - a.x).hypot(b.y - a.y)
        })
        .sum::<f32>();
    if !length.is_finite() || length <= 0.01 {
        return points.to_vec();
    }
    let phase = (f64::from(seed) / f64::from(u32::MAX) * f64::from(TAU)) as f32;
    let amplitude = amount.min(length * 0.012);
    // Keep frequency tied to identity so resizing never jumps between wave counts.
    let waves = 1.0 + (seed % 3) as f32;
    // Space samples by the wave, not by a fixed distance, so linear interpolation stays within
    // TOLERANCE of the true wobble at any length. The wobble's curvature is at most
    // |(2.2, 3.7)| * amplitude * (2pi / wavelength)^2 < 4.3 * ... (the 2w and 3w harmonics
    // dominate), and a chord of length s misses a curve of curvature k by at most k s^2 / 8;
    // 5 leaves f32 margin. The previous fixed 1 pt spacing gave a hatched 480 x 360 ellipse
    // ~19,000 segments.
    let wavelength = length / waves;
    let max_step = wavelength / TAU * (8.0 * crate::TOLERANCE / (5.0 * amplitude)).sqrt();
    let mut result = Vec::new();
    let mut distance = 0.0;
    for i in 0..count {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let segment = (b.x - a.x).hypot(b.y - a.y);
        let steps = (segment / max_step).ceil().clamp(1.0, 4096.0) as usize;
        for j in 0..steps {
            let t = j as f32 / steps as f32;
            let along = (distance + segment * t) / length;
            let envelope = if closed {
                1.0
            } else {
                (std::f32::consts::PI * along).sin()
            };
            let wave = TAU * along * waves;
            result.push(WorldPt::new(
                a.x + (b.x - a.x) * t
                    + amplitude
                        * envelope
                        * ((wave + phase).sin() + 0.3 * (wave * 2.0 - phase).sin()),
                a.y + (b.y - a.y) * t
                    + amplitude
                        * envelope
                        * ((wave - phase * 1.7).cos() + 0.3 * (wave * 3.0 + phase).sin()),
            ));
        }
        distance += segment;
    }
    if !closed && let Some(last) = points.last() {
        result.push(*last);
    }
    result
}

#[cfg(test)]
#[path = "rough_tests.rs"]
mod tests;
