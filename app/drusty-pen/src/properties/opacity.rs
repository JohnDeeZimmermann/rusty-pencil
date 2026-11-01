use super::group as field;
use crate::{State, design as d};
use pen_core::{Property, Value};
use pen_editor::{Editor, InputEvent};
use rune::{El, Key, Phase, col, el, text};
pub(super) fn group(editor: &Editor) -> El {
    let dark = editor.settings().dark;
    let value = editor.property(|p| p.opacity);
    let amount = match value {
        Value::Uniform(v) => v,
        Value::Mixed => 100,
    };
    let slider = el()
        .key("opacity")
        .w_full()
        .h(d::SLIDER_HEIGHT)
        .role("slider")
        .label("Opacity")
        .focusable(true)
        .preserve_focus_on_press(true)
        .help("Opacity · drag or use arrow keys")
        .child(
            el().absolute(0.0, (d::SLIDER_HEIGHT - d::S1) / 2.0)
                .w_full()
                .h(d::S1)
                .radius(d::S1 / 2.0)
                .fill(d::themed(d::BORDER, dark))
                .inert(),
        )
        .child(
            el().absolute(
                (d::INSPECTOR_WIDTH - 2.0 * d::S3 - d::S4) * f32::from(amount) / 100.0,
                (d::SLIDER_HEIGHT - d::S4) / 2.0,
            )
            .size(d::S4, d::S4)
            .radius(d::S4 / 2.0)
            .fill(d::themed(d::ON_ACCENT, dark))
            .inert(),
        )
        .on_drag(|s: &mut State, drag, tx| {
            if drag.phase == Phase::Down {
                s.input(InputEvent::BeginProperty, tx);
            }
            if !drag.cancelled() {
                let value = ((drag.x - d::S4 / 2.0) / (drag.w - d::S4).max(1.0) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8;
                s.input(InputEvent::Property(Property::Opacity(value)), tx);
            }
            if drag.phase == Phase::Up {
                s.input(
                    InputEvent::EndProperty {
                        cancel: drag.cancelled(),
                    },
                    tx,
                );
            }
        })
        .on_input_key(|s: &mut State, event, tx| {
            let change = match event.key {
                Key::Left | Key::Down => -1,
                Key::Right | Key::Up => 1,
                _ => return false,
            };
            let value = match s.editor.property(|p| p.opacity) {
                Value::Uniform(v) => v,
                Value::Mixed => 100,
            };
            s.input(
                InputEvent::Property(Property::Opacity(
                    (i16::from(value) + change).clamp(0, 100) as u8
                )),
                tx,
            );
            true
        });
    field(
        "Opacity",
        value == Value::Mixed,
        col().child(slider).child(
            text(format!("{amount}%"))
                .font(d::T_HINT)
                .color(d::themed(d::MUTED, dark)),
        ),
        dark,
    )
}
