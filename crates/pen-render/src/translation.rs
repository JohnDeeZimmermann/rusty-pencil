//! Recognize rigid previews without flattening their contours or copying their samples.
use pen_core::{Element, Kind, WorldPt, WorldRect};

// Translation and binding arithmetic can round differently. This is at most 0.003 screen
// points at maximum zoom; always compare to the painted snapshot so errors cannot accumulate.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.0001
}

fn same_size(a: WorldRect, b: WorldRect) -> bool {
    near(a.width(), b.width()) && near(a.height(), b.height())
}

pub(crate) fn translation(before: &Element, after: &Element) -> Option<WorldPt> {
    if before.id() != after.id()
        || before.seed() != after.seed()
        || before.frame_id != after.frame_id
        || before.props != after.props
        || before.angle.to_bits() != after.angle.to_bits()
    {
        return None;
    }
    let (a, b) = match (&before.kind, &after.kind) {
        (
            Kind::Image {
                file_id: af,
                crop: ac,
                bounds: a,
            },
            Kind::Image {
                file_id: bf,
                crop: bc,
                bounds: b,
            },
        ) if af == bf && ac == bc && same_size(*a, *b) => (a.min(), b.min()),
        (Kind::Rect(a), Kind::Rect(b))
        | (Kind::Diamond(a), Kind::Diamond(b))
        | (Kind::Ellipse(a), Kind::Ellipse(b))
            if same_size(*a, *b) =>
        {
            (a.min(), b.min())
        }
        (
            Kind::Text {
                content: ac,
                font: af,
                size: az,
                align: aa,
                bounds: a,
                container: at,
            },
            Kind::Text {
                content: bc,
                font: bf,
                size: bz,
                align: ba,
                bounds: b,
                container: bt,
            },
        ) if ac == bc
            && af == bf
            && az.to_bits() == bz.to_bits()
            && aa == ba
            && at == bt
            && same_size(*a, *b) =>
        {
            (a.min(), b.min())
        }
        (Kind::Line(a), Kind::Line(b))
        | (Kind::Arrow(a), Kind::Arrow(b))
        | (Kind::Freedraw(a), Kind::Freedraw(b))
            if a.points.len() == b.points.len()
                && a.points.iter().zip(&b.points).all(|(a, b)| {
                    near(a.offset.x, b.offset.x)
                        && near(a.offset.y, b.offset.y)
                        && a.pressure.to_bits() == b.pressure.to_bits()
                }) =>
        {
            (a.origin, b.origin)
        }
        _ => return None,
    };
    let delta = WorldPt::new(b.x - a.x, b.y - a.y);
    delta.is_finite().then_some(delta)
}
