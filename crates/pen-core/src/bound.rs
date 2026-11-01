//! Text bound to another element: wrapped inside a shape, or a label centred on a line.
//!
//! Only the text stores the link. Containers are found by scanning on commit, never per
//! frame, so the two sides can never disagree.
use std::collections::BTreeMap;

use crate::{Command, Document, Element, ElementId, Kind, Path, WorldPt, WorldRect};
use serde::{Deserialize, Serialize};

/// Inset between a shape's text box and the shape's inscribed rectangle, in world points.
pub const TEXT_PADDING: f32 = 5.0;

/// What a bound text belongs to. The variant follows from the container's kind, which never
/// changes, so it lets rendering lay text out without looking the container up.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// A rectangle, diamond or ellipse: text wraps to [`text_box`] and is centred in it.
    Shape(ElementId),
    /// A line or arrow: text keeps its measured size, centred on [`label_anchor`].
    Path(ElementId),
}

impl Container {
    /// How `element` would hold text, or `None` if it can't (text, freedraw).
    #[must_use]
    pub fn for_element(element: &Element) -> Option<Self> {
        match element.kind {
            Kind::Rect(_) | Kind::Diamond(_) | Kind::Ellipse(_) => Some(Self::Shape(element.id())),
            Kind::Line(_) | Kind::Arrow(_) => Some(Self::Path(element.id())),
            Kind::Frame { .. } | Kind::Image { .. } | Kind::Text { .. } | Kind::Freedraw(_) => None,
        }
    }

    #[must_use]
    pub fn id(self) -> ElementId {
        match self {
            Self::Shape(id) | Self::Path(id) => id,
        }
    }
}

/// Where a shape's text wraps: its inscribed rectangle, inset by [`TEXT_PADDING`]. Never
/// negative in size. `None` for kinds that are not shapes.
#[must_use]
pub fn text_box(kind: &Kind) -> Option<WorldRect> {
    let (r, inscribed) = match kind {
        Kind::Rect(r) => (r, 1.0),
        Kind::Diamond(r) => (r, 0.5),
        Kind::Ellipse(r) => (r, std::f32::consts::FRAC_1_SQRT_2),
        _ => return None,
    };
    let half_w = (r.width() * inscribed / 2.0 - TEXT_PADDING).max(0.0);
    let half_h = (r.height() * inscribed / 2.0 - TEXT_PADDING).max(0.0);
    Some(WorldRect::from_centre(r.centre(), half_w, half_h))
}

/// The shape's bounds grown downwards, if needed, so its [`text_box`] is at least `height`
/// tall. The top edge and width stay put; a shape is never shrunk.
#[must_use]
pub fn grown_to_fit(kind: &Kind, height: f32) -> Option<WorldRect> {
    let (r, inscribed) = match kind {
        Kind::Rect(r) => (r, 1.0),
        Kind::Diamond(r) => (r, 0.5),
        Kind::Ellipse(r) => (r, std::f32::consts::FRAC_1_SQRT_2),
        _ => return None,
    };
    let needed = (height + 2.0 * TEXT_PADDING) / inscribed;
    if !needed.is_finite() || needed <= r.height() + 0.01 {
        return None;
    }
    let p = r.min();
    Some(WorldRect::from_corners(
        p,
        WorldPt::new(p.x + r.width(), p.y + needed),
    ))
}

/// Halfway along the polyline through the path's vertices, in world points.
#[must_use]
pub fn label_anchor(path: &Path) -> WorldPt {
    let points: Vec<WorldPt> = path.world_points().collect();
    let length: f32 = points
        .windows(2)
        .map(|s| (s[1].x - s[0].x).hypot(s[1].y - s[0].y))
        .sum();
    let mut left = length / 2.0;
    for s in points.windows(2) {
        let d = (s[1].x - s[0].x).hypot(s[1].y - s[0].y);
        if d > 0.0 && left <= d {
            let t = left / d;
            return WorldPt::new(
                s[0].x + (s[1].x - s[0].x) * t,
                s[0].y + (s[1].y - s[0].y) * t,
            );
        }
        left -= d;
    }
    points.first().copied().unwrap_or(path.origin)
}

/// `text` placed on `container`'s current geometry. Shape text fills the text box; a label
/// keeps its size, centred on the anchor. Unbound text and mismatched pairs are unchanged.
#[must_use]
pub fn fit(text: &Element, container: &Element) -> Element {
    let Kind::Text {
        container: Some(link),
        bounds,
        ..
    } = &text.kind
    else {
        return text.clone();
    };
    let placed = match (link, &container.kind) {
        (Container::Shape(_), kind) => text_box(kind),
        (Container::Path(_), Kind::Line(path) | Kind::Arrow(path)) => {
            let curved;
            let path = if container.props.arrow_type == crate::ArrowType::Curved {
                curved = crate::routing::curved(path);
                &curved
            } else {
                path
            };
            let c = label_anchor(path);
            Some(WorldRect::from_centre(
                c,
                bounds.width() / 2.0,
                bounds.height() / 2.0,
            ))
        }
        _ => None,
    };
    let mut out = text.clone();
    out.angle = container.angle;
    out.locked = container.locked;
    out.frame_id = container.frame_id;
    if let (Some(placed), Kind::Text { bounds, .. }) = (placed, &mut out.kind) {
        let c = placed.centre();
        let to = crate::rotate(c, container.kind.bounds().centre(), container.angle);
        *bounds = placed.translated(to.x - c.x, to.y - c.y);
    }
    out
}

/// The text bound to `container`, if any. Scans the document: call on commit, not per frame.
#[must_use]
pub fn bound_text(doc: &Document, container: ElementId) -> Option<&Element> {
    doc.elements()
        .iter()
        .find(|e| e.kind.container().is_some_and(|c| c.id() == container))
}

/// The element a click on `id` acts on: a bound text's container, otherwise `id` itself.
#[must_use]
pub fn owner(doc: &Document, id: ElementId) -> ElementId {
    doc.get(id)
        .and_then(|e| e.kind.container())
        .map(Container::id)
        .filter(|c| doc.get(*c).is_some())
        .unwrap_or(id)
}

/// `command` plus the changes it implies for bound text: removing a container removes its
/// text, and changing a container's geometry re-fits its text. Text the command already
/// touches is left as the command says. Returns the command unchanged when nothing follows.
#[must_use]
pub fn follow(doc: &Document, command: Command) -> Command {
    let mut touched = BTreeMap::new();
    collect(&command, &mut touched);
    if touched.is_empty() {
        return command;
    }
    let mut extra = Vec::new();
    for text in doc.elements() {
        let Some(link) = text.kind.container() else {
            continue;
        };
        if touched.contains_key(&text.id()) {
            continue;
        }
        match touched.get(&link.id()) {
            Some(None) => extra.push(Command::Remove { id: text.id() }),
            Some(Some(after)) => {
                let after = fit(text, after);
                if after != *text {
                    extra.push(Command::Patch {
                        id: text.id(),
                        before: text.clone(),
                        after,
                    });
                }
            }
            None => {}
        }
    }
    if extra.is_empty() {
        return command;
    }
    let mut all = vec![command];
    all.extend(extra);
    Command::Group(all)
}

/// Final state per touched id: `Some` after a patch, `None` once removed.
fn collect<'a>(command: &'a Command, out: &mut BTreeMap<ElementId, Option<&'a Element>>) {
    match command {
        Command::File { .. } => {}
        Command::Group(commands) => commands.iter().for_each(|c| collect(c, out)),
        Command::Add { element, .. } => {
            out.remove(&element.id());
        }
        Command::Remove { id } => {
            out.insert(*id, None);
        }
        Command::Patch { id, after, .. } => {
            out.insert(*id, Some(after));
        }
    }
}

/// Unbinds text whose container is missing or can't hold it, so a damaged file still opens.
/// Returns how many links were dropped.
pub(crate) fn repair(elements: &mut [Element]) -> usize {
    let holders: BTreeMap<ElementId, Option<Container>> = elements
        .iter()
        .map(|e| (e.id(), Container::for_element(e)))
        .collect();
    let mut dropped = 0;
    for e in elements {
        if let Kind::Text { container, .. } = &mut e.kind
            && let Some(link) = *container
            && holders.get(&link.id()).copied().flatten() != Some(link)
        {
            *container = None;
            dropped += 1;
        }
    }
    dropped
}
