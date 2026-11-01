//! Arrow endpoint links. Focus is a local radial angle divided by π; gap is world points.
use crate::{
    ArrowType, Command, Document, Element, ElementId, Kind, Path, WorldPt, WorldRect, rotate,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Binding {
    pub element: ElementId,
    pub focus: f32,
    pub gap: f32,
}

#[must_use]
pub fn is_shape(e: &Element) -> bool {
    matches!(e.kind, Kind::Rect(_) | Kind::Diamond(_) | Kind::Ellipse(_))
}

/// Surface attachment in world coordinates, including rotation and a radial gap.
#[must_use]
pub fn anchor(shape: &Element, binding: Binding) -> WorldPt {
    let r = shape.kind.bounds();
    let c = r.centre();
    let angle = binding.focus * std::f32::consts::PI;
    let (mut dy, mut dx) = angle.sin_cos();
    if dx.abs() < 0.000001 {
        dx = 0.0;
    }
    if dy.abs() < 0.000001 {
        dy = 0.0;
    }
    let (w, h) = ((r.width() / 2.0).max(0.001), (r.height() / 2.0).max(0.001));
    let distance = match shape.kind {
        Kind::Ellipse(_) => 1.0 / ((dx / w).powi(2) + (dy / h).powi(2)).sqrt(),
        Kind::Diamond(_) => 1.0 / ((dx / w).abs() + (dy / h).abs()),
        _ => (w / dx.abs().max(0.000001)).min(h / dy.abs().max(0.000001)),
    } + binding.gap;
    rotate(
        WorldPt::new(c.x + dx * distance, c.y + dy * distance),
        c,
        shape.angle,
    )
}

#[must_use]
pub fn candidate(doc: &Document, point: WorldPt, reach: f32) -> Option<Binding> {
    doc.elements()
        .iter()
        .rev()
        .filter(|e| is_shape(e) && !e.locked)
        .find_map(|shape| {
            if !shape.hit_test_interior(point, reach) {
                return None;
            }
            let p = rotate(point, shape.kind.bounds().centre(), -shape.angle);
            let c = shape.kind.bounds().centre();
            Some(Binding {
                element: shape.id(),
                focus: (p.y - c.y).atan2(p.x - c.x) / std::f32::consts::PI,
                gap: 8.0,
            })
        })
}

/// Fits endpoints and re-routes an elbow using only its endpoint shapes.
#[must_use]
pub fn fit<'a>(arrow: &Element, lookup: impl Fn(ElementId) -> Option<&'a Element>) -> Element {
    let Kind::Arrow(path) = &arrow.kind else {
        return arrow.clone();
    };
    if path.points.len() < 2 {
        return arrow.clone();
    }
    let mut out = arrow.clone();
    let centre = path.bounds().centre();
    let mut points: Vec<_> = path
        .world_points()
        .map(|p| rotate(p, centre, arrow.angle))
        .collect();
    let mut obstacles = Vec::new();
    for (end, link) in out.bindings.iter_mut().enumerate() {
        if let Some(binding) = *link {
            if let Some(shape) = lookup(binding.element).filter(|e| is_shape(e)) {
                let i = if end == 0 { 0 } else { points.len() - 1 };
                points[i] = anchor(shape, binding);
                let r = shape.bounds();
                // Include the endpoint gap in the escape region.
                obstacles.push(WorldRect::from_corners(
                    WorldPt::new(r.min().x - 12.0, r.min().y - 12.0),
                    WorldPt::new(r.max().x + 12.0, r.max().y + 12.0),
                ));
            } else {
                *link = None;
            }
        }
    }
    if out.bindings.iter().any(Option::is_some) || arrow.props.arrow_type == ArrowType::Elbow {
        out.angle = 0.0;
        out.kind = Kind::Arrow(if arrow.props.arrow_type == ArrowType::Elbow {
            crate::routing::elbow(points[0], points[points.len() - 1], &obstacles)
        } else {
            Path::from_world(&points)
        });
    }
    out
}

/// Adds dependent patches after the initiating command, before bound text follows arrows.
#[must_use]
pub fn follow(doc: &Document, command: Command) -> Command {
    let mut touched = BTreeMap::new();
    collect(&command, &mut touched);
    let lookup = |id| {
        touched
            .get(&id)
            .map(Option::as_ref)
            .unwrap_or_else(|| doc.get(id))
    };
    let mut extra = Vec::new();
    let arrows = doc
        .elements()
        .iter()
        .filter(|e| matches!(e.kind, Kind::Arrow(_)))
        .map(Element::id)
        .chain(
            touched
                .iter()
                .filter(|(id, e)| {
                    doc.get(**id).is_none()
                        && e.as_ref().is_some_and(|e| matches!(e.kind, Kind::Arrow(_)))
                })
                .map(|(id, _)| *id),
        );
    for id in arrows {
        let Some(before) = lookup(id) else {
            continue;
        };
        if !touched.contains_key(&id)
            && !before
                .bindings
                .iter()
                .flatten()
                .any(|b| touched.contains_key(&b.element))
        {
            continue;
        }
        let after = fit(before, lookup);
        if after != *before {
            extra.push(Command::Patch {
                id,
                before: before.clone(),
                after,
            });
        }
    }
    if extra.is_empty() {
        command
    } else {
        let mut all = vec![command];
        all.extend(extra);
        Command::Group(all)
    }
}

fn collect(command: &Command, out: &mut BTreeMap<ElementId, Option<Element>>) {
    match command {
        Command::File { .. } => {}
        Command::Group(cs) => cs.iter().for_each(|c| collect(c, out)),
        Command::Add { element, .. } => {
            out.insert(element.id(), Some(element.clone()));
        }
        Command::Patch { id, after, .. } => {
            out.insert(*id, Some(after.clone()));
        }
        Command::Remove { id } => {
            out.insert(*id, None);
        }
    }
}

pub(crate) fn repair(elements: &mut [Element]) -> usize {
    let shapes: Vec<_> = elements
        .iter()
        .filter(|e| is_shape(e))
        .map(Element::id)
        .collect();
    let mut dropped = 0;
    for e in elements {
        for link in &mut e.bindings {
            if link.is_some_and(|b| {
                !matches!(e.kind, Kind::Arrow(_))
                    || !shapes.contains(&b.element)
                    || !b.focus.is_finite()
                    || !(-1.0..=1.0).contains(&b.focus)
                    || !b.gap.is_finite()
                    || !(0.0..=1000.0).contains(&b.gap)
            }) {
                *link = None;
                dropped += 1;
            }
        }
    }
    dropped
}
