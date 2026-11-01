//! Frame labels, exact geometry and local link controls.
use crate::{
    State, design as d,
    polish::{Modal, button, open},
};
use pen_core::{Kind, Link};
use pen_editor::{GeometryField, InputEvent};
use rune::{El, TextEdit, Tx, col, el, row, text};
pub(crate) fn sync(s: &mut State) {
    let element = s
        .editor
        .selection()
        .ids()
        .next()
        .and_then(|id| s.editor.document().get(id));
    if let Some(e) = element {
        if let Kind::Frame { name, .. } = &e.kind
            && !s.polish.frame.focused
        {
            s.polish.frame = TextEdit::new(name.clone());
        }
        let b = e.kind.bounds();
        for (edit, value) in s.polish.geometry.iter_mut().zip([
            b.min().x,
            b.min().y,
            b.width(),
            b.height(),
            e.angle.to_degrees(),
        ]) {
            if !edit.focused {
                *edit = TextEdit::new(format!("{value:.1}"));
            }
        }
    }
}
pub(crate) fn inspector(s: &State) -> El {
    let dark = s.editor.settings().dark;
    let Some(e) = s
        .editor
        .selection()
        .ids()
        .next()
        .and_then(|id| s.editor.document().get(id))
        .filter(|_| s.editor.selection().len() == 1)
    else {
        if s.editor.modes().stats {
            return d::surface(col().gap(d::S2), d::R_ISLAND, dark)
                .key("element-details")
                .instant_changes()
                .w(d::INSPECTOR_WIDTH)
                .padding(d::S3)
                .pin(None, Some(d::DETAILS_TOP), Some(d::S4), None)
                .child(
                    text(format!(
                        "{} elements · {} selected",
                        s.editor.document().elements().len(),
                        s.editor.selection().len()
                    ))
                    .font(d::T_LABEL)
                    .color(d::themed(d::INK, dark)),
                )
                .child(
                    text("Select one element to edit its geometry.")
                        .font(d::T_HINT)
                        .color(d::themed(d::MUTED, dark)),
                );
        }
        return el().size(0.0, 0.0);
    };
    let mut panel = col().gap(d::S2);
    if matches!(e.kind, Kind::Frame { .. }) {
        panel = panel
            .child(
                text("Frame name")
                    .font(d::T_CAPTION)
                    .color(d::themed(d::MUTED, dark)),
            )
            .child(
                rune_kit::TextInput::new(&s.polish.frame)
                    .plain()
                    .width(None)
                    .height(d::CONTROL)
                    .text_style(d::T_CONTROL)
                    .on_edit(|s: &mut State| &mut s.polish.frame)
                    .on_submit(|s: &mut State, tx| {
                        let name = s.polish.frame.text.clone();
                        s.polish.frame.focused = false;
                        s.input(InputEvent::RenameFrame(name), tx);
                        tx.blur();
                    })
                    .build(&d::kit(dark))
                    .key("frame-name")
                    .help("Enter to rename this frame"),
            );
    }
    if s.editor.modes().stats {
        panel = panel.child(
            text(format!(
                "Element #{} · {} elements",
                e.id().get(),
                s.editor.document().elements().len()
            ))
            .font(d::T_CAPTION)
            .color(d::themed(d::MUTED, dark)),
        );
        for (i, (label, field)) in [
            ("X", GeometryField::X),
            ("Y", GeometryField::Y),
            ("Width", GeometryField::Width),
            ("Height", GeometryField::Height),
            ("Angle °", GeometryField::Angle),
        ]
        .into_iter()
        .enumerate()
        {
            panel = panel.child(
                row()
                    .gap(d::S2)
                    .items_center()
                    .child(
                        text(label)
                            .w(d::STAT_LABEL)
                            .font(d::T_CAPTION)
                            .color(d::themed(d::MUTED, dark)),
                    )
                    .child(
                        rune_kit::TextInput::new(&s.polish.geometry[i])
                            .plain()
                            .width(None)
                            .height(d::CONTROL)
                            .width(Some(d::INSPECTOR_INNER - d::STAT_LABEL - d::S2))
                            .disabled(
                                s.editor.modes().view
                                    || e.locked
                                    || (field == GeometryField::Angle
                                        && matches!(e.kind, Kind::Frame { .. })),
                            )
                            .on_edit(move |s: &mut State| &mut s.polish.geometry[i])
                            .on_submit(move |s: &mut State, tx| {
                                if let Ok(value) = s.polish.geometry[i].text.parse::<f32>() {
                                    s.polish.geometry[i].focused = false;
                                    s.input(InputEvent::Geometry { field, value }, tx);
                                    tx.blur();
                                }
                            })
                            .build(&d::kit(dark))
                            .key(format!("stat-{i}")),
                    ),
            );
        }
    }
    panel = panel.child(
        button(
            if e.link.is_some() {
                "Edit link · Ctrl+K"
            } else {
                "Add link · Ctrl+K"
            },
            "link-edit",
            false,
            dark,
        )
        .on_press_tx(|s: &mut State, tx| open(s, Modal::Link, tx)),
    );
    if e.link.is_some() {
        panel = panel.child(button("Open link", "link-open", true, dark).on_press_tx(open_link));
    }
    d::surface(panel, d::R_ISLAND, dark)
        .key("element-details")
        .instant_changes()
        .w(d::INSPECTOR_WIDTH)
        .padding(d::S3)
        .pin(None, Some(d::DETAILS_TOP), Some(d::S4), None)
        .on_drag(|_: &mut State, _, tx| tx.live_only())
}
pub(crate) fn open_link(s: &mut State, tx: &mut Tx) {
    let link = s
        .editor
        .selection()
        .ids()
        .next()
        .and_then(|id| s.editor.document().get(id))
        .and_then(|e| e.link.clone());
    open_destination(s, link, tx);
}
pub(crate) fn open_destination(s: &mut State, link: Option<Link>, tx: &mut Tx) {
    match link {
        Some(Link::Web(url)) => {
            if let Err(e) = tx.open_url(&url) {
                s.files.status.set(e.to_string());
            }
        }
        Some(Link::Element(id)) => {
            if s.editor.document().get(id).is_none() {
                s.files.status.set("The linked element was removed");
                return;
            }
            s.input(InputEvent::SelectElement(id), tx);
        }
        None => {}
    }
}
