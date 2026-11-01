//! M4's small type and outline inspector. Choices apply to selection and future elements.
use crate::{State, design as d};
use pen_core::{Font, Kind, Roughness, TextAlign};
use pen_editor::{Editor, InputEvent, Tool};
use rune::{El, Tx, col, el, row, text};

pub(crate) fn panel(editor: &Editor) -> El {
    let mut panel = d::raised(col().key("properties"), d::R_ISLAND)
        .absolute(d::S4, d::INSPECTOR_TOP)
        .w(d::INSPECTOR_WIDTH)
        .padding(d::S3)
        .gap(d::S3)
        .on_drag(|_: &mut State, _, tx: &mut Tx| tx.live_only());
    let has_text = editor.tool() == Tool::Text
        || editor.text_session().is_some()
        || editor
            .selection()
            .ids()
            .filter_map(|id| editor.document().get(id))
            .any(|e| matches!(e.kind, Kind::Text { .. }));
    if has_text {
        let (font, size, align) = editor.text_style();
        panel = panel.child(label("Font"));
        for (name, value) in [
            ("Hand-drawn", Font::HandDrawn),
            ("Normal", Font::Normal),
            ("Code", Font::Code),
        ] {
            panel = panel.child(choice(
                name,
                &format!("font-{value:?}"),
                font == value,
                InputEvent::TextStyle {
                    font: value,
                    size,
                    align,
                },
            ));
        }
        panel = panel.child(label("Size"));
        let sizes = [("S", 16.0), ("M", 20.0), ("L", 28.0), ("XL", 36.0)]
            .into_iter()
            .fold(row().gap(d::S1), |r, (name, value)| {
                r.child(
                    choice(
                        name,
                        &format!("size-{name}"),
                        (size - value).abs() < 0.1,
                        InputEvent::TextStyle {
                            font,
                            size: value,
                            align,
                        },
                    )
                    .w(d::SIZE_CHOICE),
                )
            });
        panel = panel.child(sizes).child(label("Align"));
        let aligns = [
            ("Left", TextAlign::Left),
            ("Centre", TextAlign::Centre),
            ("Right", TextAlign::Right),
        ]
        .into_iter()
        .fold(row().gap(d::S1), |r, (name, value)| {
            r.child(
                choice(
                    name,
                    &format!("align-{name}"),
                    align == value,
                    InputEvent::TextStyle {
                        font,
                        size,
                        align: value,
                    },
                )
                .w(d::ALIGN_CHOICE),
            )
        });
        panel = panel.child(aligns);
    } else {
        panel = panel.child(label("Sloppiness"));
        for (name, value) in [
            ("Architect", Roughness::Architect),
            ("Artist", Roughness::Artist),
            ("Cartoonist", Roughness::Cartoonist),
        ] {
            panel = panel.child(choice(
                name,
                &format!("rough-{value:?}"),
                editor.roughness() == value,
                InputEvent::Roughness(value),
            ));
        }
    }
    panel
}

fn label(name: &str) -> El {
    text(name).font(d::T_LABEL).color(d::c(d::MUTED))
}

fn choice(name: &str, key: &str, active: bool, event: InputEvent) -> El {
    el().key(key)
        .h(d::BUTTON)
        .w_full()
        .items_center()
        .justify_center()
        .child(
            text(name)
                .font(d::T_LABEL)
                .color(d::c(if active { d::ON_ACCENT } else { d::INK })),
        )
        .radius(d::R_BUTTON)
        .color(d::c(if active { d::ON_ACCENT } else { d::INK }))
        .fill(d::c(if active { d::ACCENT } else { d::PAPER }))
        .hover(if active {
            rune::Color::TRANSPARENT
        } else {
            d::c(d::ACCENT_SOFT)
        })
        .preserve_focus_on_press(true)
        .on_press_tx(move |s: &mut State, tx| {
            s.input(event.clone(), tx);
        })
}
