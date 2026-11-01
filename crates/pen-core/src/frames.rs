//! Flat frame ownership. Frames are axis-aligned and cannot themselves belong to frames.
use crate::{Command, Document, Element, ElementId, Kind, WorldPt, WorldRect};
use std::collections::BTreeMap;

pub(crate) fn validate(elements: &[Element]) -> Result<(), crate::DocError> {
    for e in elements {
        if let Some(id) = e.frame_id
            && (matches!(e.kind, Kind::Frame { .. })
                || !elements
                    .iter()
                    .any(|f| f.id() == id && matches!(f.kind, Kind::Frame { .. })))
        {
            return Err(crate::DocError::Frame);
        }
        if matches!(e.kind, Kind::Frame { .. }) && e.angle != 0.0 {
            return Err(crate::DocError::Frame);
        }
    }
    Ok(())
}

#[must_use]
pub fn clip(doc: &Document, element: &Element) -> Option<WorldRect> {
    doc.get(element.frame_id?).map(|f| f.kind.bounds())
}

#[must_use]
pub fn visible_at(doc: &Document, element: &Element, point: WorldPt) -> bool {
    clip(doc, element).is_none_or(|b| b.contains(point))
}

/// Translates frame children, then resolves membership only at commit. A frame resize clips
/// its existing children; a child drag uses its centre to enter or leave a frame.
#[must_use]
pub fn follow(doc: &Document, mut command: Command) -> Command {
    let mut changed = BTreeMap::new();
    collect(&command, &mut changed);
    let mut finals = changed.clone();
    for e in doc.elements() {
        let Some(id) = e.frame_id else { continue };
        if changed.contains_key(&e.id()) {
            continue;
        }
        match changed.get(&id) {
            Some(None) => {
                finals.insert(e.id(), None);
            }
            Some(Some(frame)) => {
                if let Some(old) = doc.get(id) {
                    let a = old.kind.bounds().min();
                    let b = frame.kind.bounds().min();
                    if a != b
                        && (old.kind.bounds().width() - frame.kind.bounds().width()).abs() < 0.0001
                        && (old.kind.bounds().height() - frame.kind.bounds().height()).abs()
                            < 0.0001
                    {
                        finals.insert(
                            e.id(),
                            Some(e.with_bounds(e.kind.bounds().translated(b.x - a.x, b.y - a.y))),
                        );
                    }
                }
            }
            None => {}
        }
    }
    let mut frames: Vec<Element> = doc
        .elements()
        .iter()
        .filter_map(|e| finals.get(&e.id()).unwrap_or(&Some(e.clone())).clone())
        .filter(|e| matches!(e.kind, Kind::Frame { .. }))
        .collect();
    frames.extend(
        finals
            .values()
            .flatten()
            .filter(|e| doc.get(e.id()).is_none() && matches!(e.kind, Kind::Frame { .. }))
            .cloned(),
    );
    let new_frame = frames.iter().any(|f| doc.get(f.id()).is_none());
    for original in doc.elements().iter().chain(
        changed
            .values()
            .flatten()
            .filter(|e| doc.get(e.id()).is_none()),
    ) {
        let Some(mut after) = finals
            .get(&original.id())
            .unwrap_or(&Some(original.clone()))
            .clone()
        else {
            continue;
        };
        if matches!(after.kind, Kind::Frame { .. }) {
            continue;
        }
        let moved = doc
            .get(after.id())
            .is_none_or(|e| e.kind.bounds() != after.kind.bounds());
        let frame_moved = after.frame_id.is_some_and(|id| changed.contains_key(&id));
        if (moved && !frame_moved) || new_frame {
            after.frame_id = frames
                .iter()
                .rev()
                .find(|f| f.kind.bounds().contains(after.bounds().centre()))
                .map(Element::id);
        }
        if after != *original || finals.contains_key(&after.id()) {
            finals.insert(after.id(), Some(after));
        }
    }
    rewrite(
        &mut command,
        &finals,
        &mut std::collections::BTreeSet::new(),
    );
    let mut commands = vec![command];
    for (id, after) in finals {
        if changed.contains_key(&id) {
            continue;
        }
        if let Some(before) = doc.get(id) {
            commands.push(match after {
                Some(after) => Command::Patch {
                    id,
                    before: before.clone(),
                    after,
                },
                None => Command::Remove { id },
            });
        }
    }
    Command::Group(commands)
}
fn collect(command: &Command, out: &mut BTreeMap<ElementId, Option<Element>>) {
    match command {
        Command::Group(cs) => cs.iter().for_each(|c| collect(c, out)),
        Command::Patch { id, after, .. } => {
            out.insert(*id, Some(after.clone()));
        }
        Command::Add { element, .. } => {
            out.insert(element.id(), Some(element.clone()));
        }
        Command::Remove { id } => {
            out.insert(*id, None);
        }
        Command::File { .. } => {}
    }
}
fn rewrite(
    command: &mut Command,
    finals: &BTreeMap<ElementId, Option<Element>>,
    seen: &mut std::collections::BTreeSet<ElementId>,
) {
    match command {
        Command::Group(cs) => cs.iter_mut().rev().for_each(|c| rewrite(c, finals, seen)),
        Command::Patch { id, after, .. } => {
            if seen.insert(*id)
                && let Some(Some(e)) = finals.get(id)
            {
                after.clone_from(e);
            }
        }
        Command::Add { element, .. } => {
            if seen.insert(element.id())
                && let Some(Some(e)) = finals.get(&element.id())
            {
                element.clone_from(e);
            }
        }
        _ => {}
    }
}

/// Validate staged ownership without installing a partially valid command group.
pub(crate) fn validate_change(
    doc: &Document,
    staged: &BTreeMap<ElementId, Option<Element>>,
) -> bool {
    let lookup = |id| {
        staged
            .get(&id)
            .map(Option::as_ref)
            .unwrap_or_else(|| doc.get(id))
    };
    let valid = |e: &Element| {
        (!matches!(e.kind, Kind::Frame { .. }) || (e.frame_id.is_none() && e.angle == 0.0))
            && e.frame_id.is_none_or(|id| {
                !matches!(e.kind, Kind::Frame { .. })
                    && lookup(id).is_some_and(|f| matches!(f.kind, Kind::Frame { .. }))
            })
    };
    staged.values().flatten().all(valid)
        && doc
            .elements()
            .iter()
            .filter(|e| !staged.contains_key(&e.id()))
            .all(valid)
}

/// Visible geometry bounds after frame clipping; wholly clipped children have no bounds.
#[must_use]
pub fn visible_bounds(doc: &Document, e: &Element) -> Option<WorldRect> {
    let b = e.bounds();
    let Some(clip) = clip(doc, e) else {
        return Some(b);
    };
    if !b.intersects(clip) {
        return None;
    }
    Some(WorldRect::from_corners(
        WorldPt::new(b.min().x.max(clip.min().x), b.min().y.max(clip.min().y)),
        WorldPt::new(b.max().x.min(clip.max().x), b.max().y.min(clip.max().y)),
    ))
}
