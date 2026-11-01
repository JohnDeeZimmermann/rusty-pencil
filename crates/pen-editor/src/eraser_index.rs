//! Stroke-local bounds hierarchy: pointer updates query the swept area, not the document.
use pen_core::{Document, ElementId, WorldRect};

#[derive(Debug, Clone)]
pub(crate) struct EraserIndex {
    bounds: WorldRect,
    contents: Contents,
}

#[derive(Debug, Clone)]
enum Contents {
    Leaf(ElementId),
    Branch(Box<EraserIndex>, Box<EraserIndex>),
}

impl EraserIndex {
    pub(crate) fn new(doc: &Document) -> Option<Self> {
        let mut entries: Vec<_> = doc
            .elements()
            .iter()
            .filter(|e| {
                !e.locked
                    && !e
                        .frame_id
                        .and_then(|id| doc.get(id))
                        .is_some_and(|frame| frame.locked)
            })
            .map(|e| {
                let b = e.bounds();
                let pad = e.props.stroke_width * 8.0;
                (
                    WorldRect::from_centre(
                        b.centre(),
                        b.width() / 2.0 + pad,
                        b.height() / 2.0 + pad,
                    ),
                    e.id(),
                )
            })
            .collect();
        Self::build(&mut entries)
    }

    fn build(entries: &mut [(WorldRect, ElementId)]) -> Option<Self> {
        let bounds = entries.iter().map(|(b, _)| *b).reduce(WorldRect::union)?;
        let contents = if entries.len() == 1 {
            Contents::Leaf(entries[0].1)
        } else {
            let horizontal = bounds.width() >= bounds.height();
            entries.sort_by(|(a, aid), (b, bid)| {
                let a = a.centre();
                let b = b.centre();
                (if horizontal {
                    a.x.total_cmp(&b.x)
                } else {
                    a.y.total_cmp(&b.y)
                })
                .then(aid.cmp(bid))
            });
            let mid = entries.len() / 2;
            let (left, right) = entries.split_at_mut(mid);
            Contents::Branch(Box::new(Self::build(left)?), Box::new(Self::build(right)?))
        };
        Some(Self { bounds, contents })
    }

    pub(crate) fn query(&self, area: WorldRect, out: &mut Vec<ElementId>) {
        if !self.bounds.intersects(area) {
            return;
        }
        match &self.contents {
            Contents::Leaf(id) => out.push(*id),
            Contents::Branch(a, b) => {
                a.query(area, out);
                b.query(area, out);
            }
        }
    }
}
