//! Alignment treats each selected group as a unit, preserving its internal layout.
use crate::{Action, Editor, selection::active_group};
use pen_core::{Element, ElementId, GroupId, WorldRect};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UnitId {
    Group(GroupId),
    Element(ElementId),
}

fn unit(e: &Element, scope: Option<GroupId>) -> UnitId {
    active_group(e, scope).map_or(UnitId::Element(e.id()), UnitId::Group)
}

#[derive(Debug)]
struct Unit {
    elements: Vec<Element>,
    bounds: WorldRect,
}

impl Editor {
    pub(crate) fn arrange_count(&self) -> usize {
        self.selection
            .elements(&self.document)
            .map(|e| unit(e, self.group_scope))
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub(crate) fn arrange_positions(&self, action: Action, originals: &[Element]) -> Vec<Element> {
        let mut units: BTreeMap<UnitId, Unit> = BTreeMap::new();
        for e in originals {
            let u = units
                .entry(unit(e, self.group_scope))
                .or_insert_with(|| Unit {
                    elements: Vec::new(),
                    bounds: e.bounds(),
                });
            u.bounds = u.bounds.union(e.bounds());
            u.elements.push(e.clone());
        }
        let mut units: Vec<_> = units.into_values().collect();
        let Some(bounds) = units.iter().map(|u| u.bounds).reduce(WorldRect::union) else {
            return Vec::new();
        };
        let horizontal = action == Action::DistributeHorizontal;
        let distribute = matches!(
            action,
            Action::DistributeHorizontal | Action::DistributeVertical
        );
        let start = |b: WorldRect| if horizontal { b.min().x } else { b.min().y };
        let size = |b: WorldRect| if horizontal { b.width() } else { b.height() };
        let mut cursor = start(bounds);
        let gap = if distribute && units.len() >= 3 {
            units.sort_by(|a, b| start(a.bounds).total_cmp(&start(b.bounds)));
            (size(bounds) - units.iter().map(|u| size(u.bounds)).sum::<f32>())
                / (units.len() - 1) as f32
        } else {
            0.0
        };
        let mut result = Vec::with_capacity(originals.len());
        for u in units {
            let b = u.bounds;
            let (dx, dy) = match action {
                Action::AlignLeft => (bounds.min().x - b.min().x, 0.0),
                Action::AlignCentre => (bounds.centre().x - b.centre().x, 0.0),
                Action::AlignRight => (bounds.max().x - b.max().x, 0.0),
                Action::AlignTop => (0.0, bounds.min().y - b.min().y),
                Action::AlignMiddle => (0.0, bounds.centre().y - b.centre().y),
                Action::AlignBottom => (0.0, bounds.max().y - b.max().y),
                Action::DistributeHorizontal => (cursor - b.min().x, 0.0),
                _ => (0.0, cursor - b.min().y),
            };
            cursor += size(b) + gap;
            result.extend(
                u.elements
                    .into_iter()
                    .map(|e| e.with_bounds(e.kind.bounds().translated(dx, dy))),
            );
        }
        result
    }
}
