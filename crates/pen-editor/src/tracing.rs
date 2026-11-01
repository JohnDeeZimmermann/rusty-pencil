//! Ephemeral laser strokes and polygon selection never enter document history.
use crate::{Editor, Modifiers, Redraw, ScreenPt, Selection, Tool};
use pen_core::WorldPt;
#[derive(Debug, Clone)]
pub(crate) struct Trace {
    pub laser: bool,
    pub points: Vec<WorldPt>,
    before: Selection,
    additive: bool,
}
impl Editor {
    pub(crate) fn trace_down(&mut self, at: ScreenPt, mods: Modifiers) -> Option<Redraw> {
        if self.tool == Tool::Eyedropper {
            let p = self.camera.screen_to_world(at);
            let color = self
                .document
                .hit(p, 0.0, false, false)
                .map(|e| {
                    if matches!(e.kind, pen_core::Kind::Frame { .. }) {
                        return pen_core::Srgba::rgb(0x707874);
                    }
                    let mut outline = e.clone();
                    outline.props.background = None;
                    if outline.hit_test(p, 0.0) {
                        e.props.stroke
                    } else {
                        e.props.background.unwrap_or(e.props.stroke)
                    }
                })
                .unwrap_or(self.settings.canvas);
            self.set_property(if mods.shift {
                pen_core::Property::Background(Some(color))
            } else {
                pen_core::Property::Stroke(color)
            });
            self.tool = Tool::Select;
            return Some(Redraw::Scene);
        }
        if !matches!(self.tool, Tool::Laser | Tool::Lasso) {
            return None;
        }
        self.cancel();
        let p = self.camera.screen_to_world(at);
        let laser = self.tool == Tool::Laser;
        self.polish.trace = Some(Trace {
            laser,
            points: vec![p],
            before: self.selection.clone(),
            additive: mods.shift,
        });
        if laser {
            self.polish.laser.clear();
            self.polish.laser.push((p, 0.0));
        }
        Some(Redraw::Draft)
    }
    pub(crate) fn trace_move(&mut self, at: Option<ScreenPt>) -> Option<Redraw> {
        let trace = self.polish.trace.as_mut()?;
        if let Some(at) = at {
            let p = self.camera.screen_to_world(at);
            if p.is_finite()
                && trace.points.last().is_none_or(|last| {
                    (p.x - last.x).hypot(p.y - last.y) * self.camera.zoom() >= 1.0
                })
            {
                if trace.laser {
                    trace.points[0] = p;
                    self.polish.laser.push((p, 0.0));
                } else {
                    trace.points.push(p);
                }
            }
        }
        Some(Redraw::Draft)
    }
    pub(crate) fn trace_up(&mut self, at: ScreenPt, _: Modifiers) -> Option<Redraw> {
        self.trace_move(Some(at))?;
        let trace = self.polish.trace.take()?;
        if !trace.laser {
            self.selection = if trace.additive {
                trace.before
            } else {
                Selection::default()
            };
            if trace.points.len() >= 3 {
                for e in self.document.elements() {
                    if !e.locked
                        && e.kind.container().is_none()
                        && pen_core::frames::visible_at(&self.document, e, e.bounds().centre())
                        && inside(e.bounds().centre(), &trace.points)
                    {
                        self.selection
                            .expand_group(&self.document, e.id(), self.group_scope);
                    }
                }
            }
            self.tool = Tool::Select;
        }
        Some(Redraw::Tool)
    }
}
fn inside(p: WorldPt, points: &[WorldPt]) -> bool {
    let mut inside = false;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}
