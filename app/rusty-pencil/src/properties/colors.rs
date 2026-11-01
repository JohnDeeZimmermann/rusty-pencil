use super::group as field;
use crate::{State, design as d};
use pen_core::{Property, Srgba, Value};
use pen_editor::InputEvent;
use rune::{El, Key, Shape, TextEdit, canvas, col, el, row};

#[derive(Debug, Clone, Copy)]
pub(super) enum Target {
    Stroke,
    Background,
    Canvas,
}
impl Target {
    fn index(self) -> usize {
        match self {
            Self::Stroke => 0,
            Self::Background => 1,
            Self::Canvas => 2,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Stroke => "Stroke",
            Self::Background => "Background",
            Self::Canvas => "Canvas",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Canvas => "Canvas colour",
            _ => self.name(),
        }
    }
    fn value(self, s: &State) -> Value<Option<Srgba>> {
        match self {
            Self::Stroke => match s.editor.property(|p| p.stroke) {
                Value::Uniform(c) => Value::Uniform(Some(c)),
                Value::Mixed => Value::Mixed,
            },
            Self::Background => s.editor.property(|p| p.background),
            Self::Canvas => Value::Uniform(Some(s.editor.settings().canvas)),
        }
    }
    fn apply(self, s: &mut State, color: Option<Srgba>, tx: &mut rune::Tx) {
        match self {
            Self::Canvas => {
                let mut settings = s.editor.settings();
                settings.canvas = color.unwrap_or(d::CANVAS);
                s.input(InputEvent::Settings(settings), tx);
            }
            Self::Stroke => {
                s.input(
                    InputEvent::Property(Property::Stroke(
                        color.unwrap_or(Srgba { a: 0, ..Srgba::INK }),
                    )),
                    tx,
                );
            }
            Self::Background => {
                s.input(InputEvent::Property(Property::Background(color)), tx);
            }
        }
    }
}

pub(crate) fn sync(s: &mut State) {
    for target in [Target::Stroke, Target::Background, Target::Canvas] {
        if !s.hex[target.index()].focused {
            let value = match target.value(s) {
                Value::Uniform(Some(c)) => c.hex(),
                Value::Uniform(None) | Value::Mixed => String::new(),
            };
            s.hex[target.index()] = TextEdit::new(value);
        }
    }
}

pub(super) fn group(s: &State, target: Target) -> El {
    let dark = s.editor.settings().dark;
    let value = target.value(s);
    let palette = match target {
        Target::Background => [0xffe3e3, 0xd3f9d8, 0xd0ebff, 0xfff3bf, 0xe5dbff],
        Target::Canvas => [0xf8f9f7, 0xffffff, 0xfff9db, 0xe7f5ff, 0xf3f0ff],
        Target::Stroke => [0x1e1e1e, 0xe03131, 0x2f9e44, 0x1971c2, 0xf08c00],
    };
    let transparent = !matches!(target, Target::Canvas);
    let mut controls = row().gap(d::S1);
    if transparent {
        let active = matches!(
            value,
            Value::Uniform(None) | Value::Uniform(Some(Srgba { a: 0, .. }))
        );
        let chip = canvas(no_colour())
            .size(d::SWATCH_CHIP, d::SWATCH_CHIP)
            .radius(d::S1)
            .fill(d::themed(d::PAPER, dark))
            .border(d::OUTLINE, d::themed(d::BORDER, dark))
            .inert();
        controls = controls.child(
            swatch(format!("{}-transparent", target.name()), active, dark)
                .help("Transparent")
                .child(chip)
                .on_press_tx(move |s: &mut State, tx| target.apply(s, None, tx)),
        );
    }
    for (i, rgb) in palette.into_iter().enumerate() {
        let color = Srgba::rgb(rgb);
        let active = value == Value::Uniform(Some(color));
        controls = controls.child(
            swatch(format!("{}-color-{i}", target.name()), active, dark)
                .help(color.hex())
                .child(
                    el().size(d::SWATCH_CHIP, d::SWATCH_CHIP)
                        .radius(d::S1)
                        .fill(d::themed(color, dark))
                        .border(d::OUTLINE, d::themed(d::BORDER, dark))
                        .inert(),
                )
                .on_press_tx(move |s: &mut State, tx| target.apply(s, Some(color), tx)),
        );
    }
    let edit = &s.hex[target.index()];
    let invalid = edit.focused && !edit.text.is_empty() && Srgba::from_hex(&edit.text).is_none();
    let focused = edit.focused;
    let input = rune_kit::TextInput::new(edit)
        .plain()
        .width(None)
        .height(d::CONTROL)
        .text_style(d::T_CONTROL)
        .placeholder(if value == Value::Mixed {
            "Mixed"
        } else {
            "#RRGGBB"
        })
        .invalid(invalid)
        .on_edit(move |s: &mut State| &mut s.hex[target.index()])
        .on_submit(move |s: &mut State, tx| {
            if let Some(color) = Srgba::from_hex(&s.hex[target.index()].text) {
                target.apply(s, Some(color), tx);
                tx.blur();
            }
        })
        .on_key(move |s: &mut State, event, tx| {
            if event.key == Key::Escape {
                if matches!(target, Target::Canvas) {
                    s.menu = None;
                }
                s.hex[target.index()].focused = false;
                sync(s);
                tx.blur();
                return true;
            }
            false
        })
        .build(&d::kit(dark))
        .key(format!("{}-hex", target.name()))
        .help("Custom hex colour · Enter to apply");
    // Flat frame matching the segmented tracks; the teal border is the focus ring.
    let input = row()
        .w_full()
        .px(d::S1)
        .radius(d::R_BUTTON)
        .fill(d::themed(d::ACCENT_SOFT, dark))
        .border(
            d::OUTLINE,
            d::themed(
                if focused {
                    d::ON_ACCENT
                } else {
                    d::ACCENT_SOFT
                },
                dark,
            ),
        )
        .child(input);
    let group = col().gap(d::S_LABEL).child(controls).child(input);
    field(target.label(), value == Value::Mixed, group, dark)
}

/// Square target around a colour chip; the active one gets the teal wash.
fn swatch(key: String, active: bool, dark: bool) -> El {
    let base = el()
        .key(key)
        .size(d::SWATCH, d::SWATCH)
        .radius(d::R_CONTROL)
        .items_center()
        .justify_center()
        .preserve_focus_on_press(true)
        .press_scale(d::PRESS);
    if active {
        base.fill(d::themed(d::ACCENT, dark))
            .border(d::OUTLINE, d::themed(d::ON_ACCENT, dark))
    } else {
        base.hover(d::themed(d::ACCENT_SOFT, dark))
    }
}

/// A diagonal slash across the chip: the conventional "no colour" mark.
fn no_colour() -> Shape {
    let inset = 3.0;
    Shape::new().stroke(
        vec![
            [inset, d::SWATCH_CHIP - inset],
            [d::SWATCH_CHIP - inset, inset],
        ],
        1.5,
        d::c(Srgba::rgb(0xe0_31_31)),
    )
}
