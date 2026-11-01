//! Screen-space selection feedback, independent of document paint and its geometry cache.
use pen_core::WorldRect;
use pen_editor::{Editor, HANDLE_SIZE, Handle, SELECTION_PADDING, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Outline,
    Handle(Handle),
    Marquee,
    Vertex { active: bool },
    Midpoint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayRect {
    pub kind: OverlayKind,
    /// Screen points relative to the viewport, [x, y, width, height].
    pub rect: [f32; 4],
}

/// Reuses `out`. Camera changes only update the presentation of these screen rectangles;
/// document paint stays cached. Handles work for single and multi-element selections.
pub fn overlay(editor: &Editor, out: &mut Vec<OverlayRect>) {
    out.clear();
    for bounds in editor.binding_targets() {
        let p = editor.camera().world_to_screen(bounds.min());
        out.push(OverlayRect {
            kind: OverlayKind::Marquee,
            rect: [
                p.x - 4.0,
                p.y - 4.0,
                bounds.width() * editor.camera().zoom() + 8.0,
                bounds.height() * editor.camera().zoom() + 8.0,
            ],
        });
    }
    if editor.tool() != Tool::Select {
        return;
    }
    let camera = editor.camera();
    if let Some((path, active)) = editor.editable_points() {
        let (centre, angle) = editor
            .point_edit_transform()
            .unwrap_or((pen_core::WorldPt::ORIGIN, 0.0));
        let mut previous = None;
        for (i, point) in path.world_points().enumerate() {
            let p = camera.world_to_screen(pen_core::rotate(point, centre, angle));
            if let Some(a) = previous {
                let a: pen_editor::ScreenPt = a;
                out.push(OverlayRect {
                    kind: OverlayKind::Midpoint,
                    rect: [(a.x + p.x) / 2.0 - 3.0, (a.y + p.y) / 2.0 - 3.0, 6.0, 6.0],
                });
            }
            out.push(OverlayRect {
                kind: OverlayKind::Vertex {
                    active: active == Some(i),
                },
                rect: [
                    p.x - HANDLE_SIZE / 2.0,
                    p.y - HANDLE_SIZE / 2.0,
                    HANDLE_SIZE,
                    HANDLE_SIZE,
                ],
            });
            previous = Some(p);
        }
        return;
    }
    let screen = |r: WorldRect, pad: f32| {
        let a = camera.world_to_screen(r.min());
        [
            a.x - pad,
            a.y - pad,
            r.width() * camera.zoom() + pad * 2.0,
            r.height() * camera.zoom() + pad * 2.0,
        ]
    };
    if let Some(bounds) = editor.selection_bounds() {
        out.push(OverlayRect {
            kind: OverlayKind::Outline,
            rect: screen(bounds, SELECTION_PADDING),
        });
        if !editor.selection().is_empty()
            && editor.marquee().is_none()
            && !editor.selection_locked()
        {
            for handle in Handle::ALL.into_iter().chain([Handle::Rotate]) {
                let p = camera.world_to_screen(handle.position(bounds, camera.zoom()));
                out.push(OverlayRect {
                    kind: OverlayKind::Handle(handle),
                    rect: [
                        p.x - HANDLE_SIZE / 2.0,
                        p.y - HANDLE_SIZE / 2.0,
                        HANDLE_SIZE,
                        HANDLE_SIZE,
                    ],
                });
            }
        }
    }
    if let Some(bounds) = editor.marquee() {
        out.push(OverlayRect {
            kind: OverlayKind::Marquee,
            rect: screen(bounds, 0.0),
        });
    }
}
