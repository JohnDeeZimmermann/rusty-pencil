//! Constant-screen-size feedback presented through live rectangles: pan/zoom needs no view
//! rebuild. The renderer owns geometry; design tokens own every visual choice.
use crate::design::{self as d, c};
use pen_editor::{Editor, Handle};
use pen_render::{OverlayKind, OverlayRect};
use rune::{Cursor, El, Live, el};

struct Item {
    kind: OverlayKind,
    rect: Live<[f32; 4]>,
}

#[derive(Debug, Default)]
pub(crate) struct Overlay {
    items: Vec<Item>,
    scratch: Vec<OverlayRect>,
}

impl Overlay {
    pub(crate) fn show(&mut self, editor: &Editor) {
        pen_render::overlay(editor, &mut self.scratch);
        if self.items.len() != self.scratch.len()
            || self
                .items
                .iter()
                .zip(&self.scratch)
                .any(|(a, b)| a.kind != b.kind)
        {
            self.items = self
                .scratch
                .iter()
                .map(|r| Item {
                    kind: r.kind,
                    rect: Live::new(r.rect),
                })
                .collect();
        } else {
            for (item, rect) in self.items.iter().zip(&self.scratch) {
                item.rect.set(rect.rect);
            }
        }
    }

    pub(crate) fn view(&self) -> El {
        self.items.iter().enumerate().fold(
            el().key("selection-overlay")
                .absolute(0.0, 0.0)
                .size(0.0, 0.0),
            |layer, (i, item)| {
                let rect = el()
                    .key(format!("selection-{i}"))
                    .absolute(0.0, 0.0)
                    .size(0.0, 0.0)
                    .live_rect(&item.rect)
                    .border(d::OUTLINE, c(d::ON_ACCENT))
                    .transition(rune::Transition::none());
                let rect = match item.kind {
                    OverlayKind::Outline => rect.cursor(Cursor::Move),
                    OverlayKind::Marquee => rect.fill(c(d::SELECTION_WASH)),
                    OverlayKind::Vertex { active } => rect
                        .fill(c(if active { d::ON_ACCENT } else { d::PAPER }))
                        .radius(d::R_STATUS)
                        .cursor(Cursor::Move),
                    OverlayKind::Midpoint => rect
                        .fill(c(d::ACCENT))
                        .radius(d::R_STATUS)
                        .cursor(Cursor::Crosshair),
                    OverlayKind::Handle(handle) => rect
                        .fill(c(d::PAPER))
                        .radius(d::R_HANDLE)
                        .cursor(match handle {
                            Handle::N | Handle::S => Cursor::ResizeRow,
                            Handle::E | Handle::W => Cursor::ResizeColumn,
                            _ => Cursor::Crosshair,
                        }),
                };
                layer.child(rect)
            },
        )
    }
}

impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Item")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}
