//! The font engine measures world geometry; Rune's field owns caret, selection and IME.
use crate::{State, design as d, layer, paint as shapes};
use pen_core::{Container, Font, Kind, TextAlign, WorldPt, WorldRect};
use pen_editor::InputEvent;
use pen_render::{Prim, TextFit};
use rune::text::{RichLayoutOptions, TextSystem};
use rune::{Align, El, Family, Key, TextEdit, Tx, col, el, text, text_input, tok::TextStyle};

pub(crate) static FONTS: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/Inter.ttf"),
    include_bytes!("../../../assets/fonts/PatrickHand.ttf"),
    include_bytes!("../../../assets/fonts/RobotoMono.ttf"),
];
// Rune bounds inherited glyph raster scale to 1/16..=4. Give document text an
// equivalent 8x layout with a compensating transform, so 3000% reaches only 3.75x.
// This preserves live-only camera updates and rasterizes at the actual screen size.
const TEXT_LAYOUT_SCALE: f32 = 8.0;

fn display_style(font: Font, size: f32) -> TextStyle {
    style(font, size * TEXT_LAYOUT_SCALE).optical_size(size)
}

pub(crate) const FAMILIES: (&str, &str) = ("Inter", "Roboto Mono");

pub(crate) fn system() -> TextSystem {
    TextSystem::new(FONTS, FAMILIES)
}

pub(crate) fn style(font: Font, size: f32) -> TextStyle {
    TextStyle::new(size, 400.0)
        .lh(size * 1.25)
        .family(Family::Named(match font {
            Font::HandDrawn => "Patrick Hand",
            Font::Normal => "Inter",
            Font::Code => "Roboto Mono",
        }))
}
fn alignment(align: TextAlign) -> Align {
    match align {
        TextAlign::Left => Align::Start,
        TextAlign::Centre => Align::Center,
        TextAlign::Right => Align::End,
    }
}

/// Laid-out `(width, height)` in world points. With `wrap`, lines break at that width
/// exactly as [`paint_with_images`] and the editing overlay break them.
pub(crate) fn measure(
    system: &mut TextSystem,
    content: &str,
    font: Font,
    size: f32,
    wrap: Option<f32>,
) -> (f32, f32) {
    if let Some(width) = wrap {
        let style = display_style(font, size);
        let layout = system.layout_rich_with(
            content,
            &[],
            style,
            1.0,
            Some(width.max(1.0) * TEXT_LAYOUT_SCALE),
            RichLayoutOptions::new().wrap_anywhere(),
        );
        let height = rune::text::layout_height(&layout, style, 1.0);
        // `width()` leaves out hanging trailing spaces, which may overhang the wrap width.
        let visible = layout.width();
        system.end_commit();
        return (
            visible.ceil() / TEXT_LAYOUT_SCALE,
            height / TEXT_LAYOUT_SCALE,
        );
    }
    let width = content
        .split('\n')
        .map(|line| system.measure(line, style(font, size), 1.0).0)
        .fold(1.0_f32, f32::max);
    system.end_commit();
    (
        width + 2.0,
        content.split('\n').count() as f32 * size * 1.25,
    )
}

/// Room around a label's backdrop, in world points.
const LABEL_MARGIN: f32 = 4.0;

/// A text element at 8x layout scale with a compensating transform; see [`TEXT_LAYOUT_SCALE`].
fn placed(base: El, bounds: WorldRect) -> El {
    base.absolute(bounds.min().x, bounds.min().y)
        .size(
            bounds.width() * TEXT_LAYOUT_SCALE,
            bounds.height() * TEXT_LAYOUT_SCALE,
        )
        .scale(1.0 / TEXT_LAYOUT_SCALE)
        .origin(0.0, 0.0)
}

fn backdrop(bounds: WorldRect, settings: pen_core::Settings) -> El {
    let m = LABEL_MARGIN;
    el().absolute(bounds.min().x - m, bounds.min().y - m)
        .size(bounds.width() + 2.0 * m, bounds.height() + 2.0 * m)
        .radius(LABEL_MARGIN)
        .fill(d::themed(settings.canvas, settings.dark))
}

/// Partition around text so filled shapes still occlude text below them in document order.
pub(crate) fn paint_with_images(
    prims: &[Prim],
    settings: pen_core::Settings,
    images: &crate::images::Cache,
) -> El {
    let themed;
    let prims = if settings.dark {
        themed = crate::paint::themed(prims);
        &themed
    } else {
        prims
    };
    let mut root = el()
        .absolute(0.0, 0.0)
        .size(0.0, 0.0)
        .instant_changes()
        .inert();
    let mut start = 0;
    for (i, prim) in prims.iter().enumerate() {
        if matches!(prim, Prim::Image { .. }) {
            if start < i {
                let (shape, bounds) = shapes::bounded(&prims[start..i]);
                root = root.child(layer(shape, bounds, &format!("paint-{start}")));
            }
            root = root.child(images.view(prim).key(format!("image-{i}")));
            start = i + 1;
        } else if let Prim::Text {
            angle,
            content,
            font,
            size,
            align,
            bounds,
            color,
            fit,
            vertical_align,
        } = prim
        {
            if start < i {
                let (shape, bounds) = shapes::bounded(&prims[start..i]);
                root = root.child(layer(shape, bounds, &format!("paint-{start}")));
            }
            let label = text(content.clone())
                .font(display_style(*font, *size))
                .align(alignment(*align))
                .color(d::c(*color));
            let mut group = el()
                .absolute(bounds.min().x, bounds.min().y)
                .size(bounds.width(), bounds.height())
                .rotate(*angle)
                .instant_changes();
            let local = WorldRect::from_corners(
                WorldPt::ORIGIN,
                WorldPt::new(bounds.width(), bounds.height()),
            );
            group = match fit {
                TextFit::Free => group.child(placed(label, local).key(format!("text-{i}"))),
                TextFit::Wrap => group.child(
                    vertical(placed(col(), local), *vertical_align)
                        .key(format!("text-{i}"))
                        .child(label.w_full().wrap_anywhere()),
                ),
                TextFit::Label => group
                    .child(backdrop(local, settings).key(format!("backdrop-{i}")))
                    .child(placed(label, local).key(format!("text-{i}"))),
            };
            root = root.child(group);
            start = i + 1;
        }
    }
    if start < prims.len() {
        let (shape, bounds) = shapes::bounded(&prims[start..]);
        root = root.child(layer(shape, bounds, &format!("paint-{start}")));
    }
    root
}

pub(crate) fn editor(s: &State) -> El {
    let Some(session) = s.editor.text_session() else {
        return el().size(0.0, 0.0);
    };
    if session.element.angle == 0.0 {
        return editor_inner(s);
    }
    let b = session.element.kind.bounds();
    el().key("rotated-text-editor")
        .absolute(b.min().x, b.min().y)
        .size(b.width(), b.height())
        .rotate(session.element.angle)
        .instant_changes()
        .child(
            el().absolute(-b.min().x, -b.min().y)
                .size(0.0, 0.0)
                .child(editor_inner(s)),
        )
}

fn editor_inner(s: &State) -> El {
    let Some(session) = s.editor.text_session() else {
        return el().size(0.0, 0.0);
    };
    let Kind::Text {
        font,
        size,
        align,
        bounds,
        container,
        ..
    } = &session.element.kind
    else {
        return el();
    };
    let settings = s.editor.settings();
    let style = display_style(*font, *size);
    let field = text_input(&s.text_edit, style)
        .key("canvas-text-editor")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .instant_changes()
        .color(d::themed(session.element.props.stroke, settings.dark))
        .opacity(f32::from(session.element.props.opacity) / 100.0)
        .caret(
            d::themed(d::ON_ACCENT, settings.dark),
            d::themed(d::SELECTION_WASH, settings.dark),
        )
        .align(alignment(*align))
        .on_edit(|s: &mut State| &mut s.text_edit)
        .on_input_change(|s: &mut State, tx| s.sync_text(tx))
        .on_submit(|s: &mut State, tx| {
            s.text_edit.insert("\n");
            s.sync_text(tx);
        })
        .on_input_key(|s: &mut State, event, tx| {
            if event.key == Key::Escape {
                s.sync_text(tx);
                s.input(InputEvent::FinishText, tx);
                tx.blur();
                return true;
            }
            if crate::export::key(s, event, tx) {
                return true;
            }
            if crate::files::key(s, event, tx) {
                return true;
            }
            if event.mods.cmd
                && event.mods.shift
                && matches!(
                    event.key,
                    Key::Command('<' | '>' | ',' | '.') | Key::Text(_)
                )
            {
                return crate::on_key(s, event, tx);
            }
            false
        });
    // The invisible label lets Rune lay out the full IME preedit in this frame, before it
    // becomes committed content. The overlay field shares its intrinsic size and position.
    let sizer = text(s.text_edit.display().0)
        .font(style)
        .align(alignment(*align))
        .color(rune::Color::TRANSPARENT)
        .inert();
    if matches!(container, Some(Container::Shape(_))) {
        // Wrapped at the container's text box and centred in it, as it will be painted.
        return vertical(placed(col(), *bounds), session.element.props.vertical_align)
            .key("text-edit-layout")
            .instant_changes()
            .child(
                el().w_full()
                    .child(sizer.w_full().wrap_anywhere())
                    .child(field.wrap_anywhere()),
            );
    }
    let layout = el()
        .key("text-edit-layout")
        .absolute(bounds.min().x, bounds.min().y)
        .min_w(bounds.width().max(3.0) * TEXT_LAYOUT_SCALE)
        .min_h(bounds.height().max(*size * 1.25) * TEXT_LAYOUT_SCALE)
        .scale(1.0 / TEXT_LAYOUT_SCALE)
        .origin(0.0, 0.0)
        .instant_changes()
        .child(sizer)
        .child(field);
    if matches!(container, Some(Container::Path(_))) {
        // Mask the labelled line while typing, as the committed label will.
        return el()
            .key("text-edit-label")
            .absolute(0.0, 0.0)
            .size(0.0, 0.0)
            .instant_changes()
            .child(backdrop(*bounds, settings).key("text-edit-backdrop"))
            .child(layout);
    }
    layout
}

impl State {
    pub(crate) fn sync_text(&mut self, tx: &mut Tx) {
        let Some(session) = self.editor.text_session() else {
            return;
        };
        let Kind::Text {
            font, size, bounds, ..
        } = session.element.kind
        else {
            return;
        };
        let wrap = session
            .container()
            .and_then(|c| pen_core::bound::text_box(&c.kind))
            .map(|r| r.width());
        // Composition remains in TextEdit until committed by the platform IME.
        let (w, h) = measure(&mut self.fonts, &self.text_edit.text, font, size, wrap);
        let p = bounds.min();
        self.input(
            InputEvent::TextChanged {
                content: self.text_edit.text.clone(),
                bounds: WorldRect::from_corners(p, WorldPt::new(p.x + w, p.y + h)),
            },
            tx,
        );
    }

    pub(crate) fn text_session_changed(&mut self, old: Option<pen_core::ElementId>) {
        let session = self.editor.text_session();
        if session.map(|s| s.element.id()) == old {
            return;
        }
        self.text_edit = session.map_or_else(TextEdit::default, |session| {
            let Kind::Text { content, .. } = &session.element.kind else {
                return TextEdit::default();
            };
            let mut edit = TextEdit::new(content.clone());
            edit.focused = true;
            edit
        });
    }
}

fn vertical(el: El, align: pen_core::VerticalAlign) -> El {
    match align {
        pen_core::VerticalAlign::Top => el,
        pen_core::VerticalAlign::Middle => el.justify_center(),
        pen_core::VerticalAlign::Bottom => el.justify_end(),
    }
}
