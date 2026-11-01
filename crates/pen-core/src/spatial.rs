//! Deterministic bounds hierarchy and sorted alignment anchors, rebuilt only on commands.
use crate::{Element, ElementId, Kind, WorldRect};

#[derive(Debug, Clone)]
struct Node {
    bounds: WorldRect,
    contents: Contents,
}
#[derive(Debug, Clone)]
enum Contents {
    Leaf(usize),
    Branch(Box<Node>, Box<Node>),
}
/// A bound used for object alignment, in world points.
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub value: f32,
    pub id: ElementId,
    pub bounds: WorldRect,
}
#[derive(Debug, Clone, Default)]
pub struct SpatialIndex {
    root: Option<Node>,
    axes: [Vec<Anchor>; 2],
}
impl SpatialIndex {
    pub(crate) fn build(elements: &[Element]) -> Self {
        let mut entries: Vec<_> = elements
            .iter()
            .enumerate()
            .map(|(at, e)| {
                let b = e.bounds();
                let pad = e.props.stroke_width * 8.0 + 24.0;
                (
                    WorldRect::from_centre(
                        b.centre(),
                        b.width() / 2.0 + pad,
                        b.height() / 2.0 + pad,
                    ),
                    at,
                )
            })
            .collect();
        let mut axes: [Vec<Anchor>; 2] = Default::default();
        for e in elements
            .iter()
            .filter(|e| e.kind.container().is_none() && !matches!(e.kind, Kind::Frame { .. }))
        {
            let b = e.bounds();
            for (axis, values) in [
                [b.min().x, b.centre().x, b.max().x],
                [b.min().y, b.centre().y, b.max().y],
            ]
            .into_iter()
            .enumerate()
            {
                axes[axis].extend(values.into_iter().map(|value| Anchor {
                    value,
                    id: e.id(),
                    bounds: b,
                }));
            }
        }
        for axis in &mut axes {
            axis.sort_by(|a, b| a.value.total_cmp(&b.value).then(a.id.cmp(&b.id)));
        }
        Self {
            root: Node::build(&mut entries),
            axes,
        }
    }
    /// Visit document z-positions intersecting an area, without allocating a result buffer.
    pub fn visit(&self, area: WorldRect, mut visitor: impl FnMut(usize)) {
        if let Some(root) = &self.root {
            root.visit(area, &mut visitor);
        }
    }
    #[must_use]
    pub fn anchors(&self, vertical: bool) -> &[Anchor] {
        &self.axes[usize::from(vertical)]
    }
}
impl Node {
    fn build(entries: &mut [(WorldRect, usize)]) -> Option<Self> {
        let bounds = entries.iter().map(|(b, _)| *b).reduce(WorldRect::union)?;
        let contents = if entries.len() == 1 {
            Contents::Leaf(entries[0].1)
        } else {
            let horizontal = bounds.width() >= bounds.height();
            entries.sort_by(|(a, ai), (b, bi)| {
                let a = a.centre();
                let b = b.centre();
                (if horizontal {
                    a.x.total_cmp(&b.x)
                } else {
                    a.y.total_cmp(&b.y)
                })
                .then(ai.cmp(bi))
            });
            let mid = entries.len() / 2;
            let (a, b) = entries.split_at_mut(mid);
            Contents::Branch(Box::new(Self::build(a)?), Box::new(Self::build(b)?))
        };
        Some(Self { bounds, contents })
    }
    fn visit(&self, area: WorldRect, visitor: &mut impl FnMut(usize)) {
        if !self.bounds.intersects(area) {
            return;
        }
        match &self.contents {
            Contents::Leaf(at) => visitor(*at),
            Contents::Branch(a, b) => {
                a.visit(area, visitor);
                b.visit(area, visitor);
            }
        }
    }
}
