//! Image placement and non-destructive cropping share the normal command history.
use crate::{Editor, Redraw, ScreenPt, Selection, Tool};
use pen_core::{BinaryFile, Command, Crop, Element, Kind, Props, WorldPt, WorldRect};
impl Editor {
    pub(crate) fn insert_image(&mut self, file: BinaryFile, at: ScreenPt) -> Redraw {
        if !file.valid() || !at.x.is_finite() || !at.y.is_finite() {
            return Redraw::Nothing;
        }
        self.finish_text();
        self.cancel();
        let before = self.selection.clone();
        let p = self.camera.screen_to_world(at);
        let scale = (480.0 / file.width.max(file.height) as f32).min(1.0);
        let bounds = WorldRect::from_corners(
            p,
            WorldPt::new(
                p.x + file.width as f32 * scale,
                p.y + file.height as f32 * scale,
            ),
        );
        let file_id = file.id();
        let id = self.document.alloc_id();
        let element = Element::new(
            id,
            Kind::Image {
                file_id: file_id.clone(),
                crop: Crop::default(),
                bounds,
            },
            Props::default(),
        );
        let mut commands = Vec::new();
        if !self.document.files().contains_key(&file_id) {
            commands.push(Command::File {
                id: file_id,
                file: Some(file),
            });
        }
        commands.push(Command::Add {
            element,
            at: self.document.elements().len(),
        });
        self.selection = Selection::only(id);
        if self.execute(Command::Group(commands), before.clone()) {
            self.tool = Tool::Select;
            Redraw::Scene
        } else {
            self.selection = before;
            Redraw::Nothing
        }
    }
    pub(crate) fn crop_image(&mut self, crop: Crop) -> Redraw {
        if !crop.valid() {
            return Redraw::Nothing;
        }
        let mut commands = Vec::new();
        for before in self.selection.elements(&self.document) {
            if before.locked {
                continue;
            }
            if let Kind::Image { crop: old, .. } = &before.kind {
                if *old == crop {
                    continue;
                }
                let mut after = before.clone();
                if let Kind::Image {
                    crop: target,
                    bounds,
                    ..
                } = &mut after.kind
                {
                    let b = *bounds;
                    let min = WorldPt::new(
                        b.min().x + (crop.x - old.x) / old.width * b.width(),
                        b.min().y + (crop.y - old.y) / old.height * b.height(),
                    );
                    let next = WorldRect::from_corners(
                        min,
                        WorldPt::new(
                            min.x + b.width() * crop.width / old.width,
                            min.y + b.height() * crop.height / old.height,
                        ),
                    );
                    let centre = pen_core::rotate(next.centre(), b.centre(), before.angle);
                    *bounds =
                        next.translated(centre.x - next.centre().x, centre.y - next.centre().y);
                    *target = crop;
                }
                commands.push(Command::Patch {
                    id: before.id(),
                    before: before.clone(),
                    after,
                });
            }
        }
        if !commands.is_empty() && self.execute(Command::Group(commands), self.selection.clone()) {
            Redraw::Scene
        } else {
            Redraw::Nothing
        }
    }
}
