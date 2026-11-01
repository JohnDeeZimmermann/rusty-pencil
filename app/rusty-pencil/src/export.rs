//! Local export choices and clipboard transport, using the same world-space draw list.
use crate::{State, design as d};
use pen_render::{
    DrawList,
    export::{self, Options},
};
use rune::{
    ClipboardContent, ClipboardImage, El, FileDialogOptions, Key, KeyEvent, Tx, col, el, row, text,
};
#[derive(Debug)]
pub(crate) struct Panel {
    pub(crate) open: bool,
    scale: u8,
    background: bool,
    dark: bool,
    pub(crate) selection: bool,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            open: false,
            scale: 1,
            background: true,
            dark: false,
            selection: false,
        }
    }
}
fn paint(s: &State, selection: bool) -> DrawList {
    let mut list = DrawList::new();
    let frame = selection
        .then(|| {
            s.editor
                .selection()
                .ids()
                .next()
                .filter(|_| s.editor.selection().len() == 1)
                .and_then(|id| s.editor.document().get(id))
                .filter(|e| matches!(e.kind, pen_core::Kind::Frame { .. }))
        })
        .flatten();
    if let Some(frame) = frame {
        let mut inner = DrawList::new();
        for e in s
            .editor
            .document()
            .elements()
            .iter()
            .filter(|e| e.frame_id == Some(frame.id()))
        {
            pen_render::draw_element(e, &mut inner);
        }
        list.push(pen_render::Prim::Clip {
            bounds: frame.kind.bounds(),
            prims: inner.prims().to_vec(),
        });
        return list;
    }
    for e in s.editor.document().elements() {
        if !selection
            || s.editor.selection().contains(e.id())
            || e.frame_id
                .is_some_and(|id| s.editor.selection().contains(id))
            || e.kind
                .container()
                .is_some_and(|c| s.editor.selection().contains(c.id()))
        {
            pen_render::frames::draw(s.editor.document(), &[], e, &mut list);
        }
    }
    list
}
fn bytes(s: &State, svg: bool, selection: bool) -> Result<Vec<u8>, String> {
    let options = Options {
        scale: s.export.scale,
        background: s.export.background.then_some(s.editor.settings().canvas),
        dark: s.export.dark,
    };
    let paint = paint(s, selection);
    if svg {
        export::svg(&paint, s.editor.document().files(), options).map(String::into_bytes)
    } else {
        export::png(&paint, s.editor.document().files(), options)
    }
    .map_err(|e| e.to_string())
}
fn save(s: &mut State, svg: bool, tx: &mut Tx) {
    let data = match bytes(s, svg, s.export.selection) {
        Ok(data) => data,
        Err(e) => {
            s.files.status.set(e);
            return;
        }
    };
    let extension = if svg { "svg" } else { "png" };
    crate::platform::save_file(
        s,
        tx,
        FileDialogOptions {
            title: "Export drawing".into(),
            file_name: Some(format!("Drawing.{extension}")),
            ..Default::default()
        }
        .filter("Drawing", &[extension]),
        move |s, result, _| match result {
            Ok(Some(path)) => match crate::platform::deliver(&path, &data) {
                Ok(()) => {
                    s.files.status.set("Exported drawing");
                    s.export.open = false;
                }
                Err(e) => s.files.status.set(format!("Could not export: {e}")),
            },
            Ok(None) => {}
            Err(e) => s.files.status.set(format!("Could not export: {e}")),
        },
    );
}
fn copy(s: &mut State, svg: bool, tx: &mut Tx) {
    if s.editor.selection().is_empty() {
        s.files.status.set("Select elements to copy as an image");
        return;
    }
    match bytes(s, svg, true) {
        Ok(data) => {
            if svg {
                let svg = String::from_utf8_lossy(&data).into_owned();
                tx.copy_rich(ClipboardContent::new(svg.clone()).with_html(svg));
            } else if let Some(image) = ClipboardImage::from_png(data) {
                tx.copy_rich(ClipboardContent::new("").with_image(image));
            } else {
                s.files.status.set("Image is too large for the clipboard");
                return;
            }
            s.files.status.set(if svg {
                "Copied selection as SVG"
            } else {
                "Copied selection as PNG"
            });
        }
        Err(e) => s.files.status.set(e),
    }
}
pub(crate) fn key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key == Key::Escape && s.export.open {
        s.export.open = false;
        return true;
    }
    if !ev.mods.cmd || !ev.mods.shift {
        return false;
    }
    let c = match &ev.key {
        Key::Command(c) => *c,
        Key::Text(s) => s.chars().next().unwrap_or('\0'),
        _ => return false,
    };
    match c.to_ascii_lowercase() {
        'e' => {
            if s.editor.text_session().is_some() {
                s.sync_text(tx);
                s.input(pen_editor::InputEvent::FinishText, tx);
            }
            s.export.open = !s.export.open;
            if s.export.open {
                s.export.selection = s.editor.selection().len() == 1
                    && s.editor
                        .selection()
                        .ids()
                        .next()
                        .and_then(|id| s.editor.document().get(id))
                        .is_some_and(|e| matches!(e.kind, pen_core::Kind::Frame { .. }));
            }
            s.menu = None;
        }
        'c' => copy(s, ev.mods.alt, tx),
        _ => return false,
    }
    true
}
fn button(label: &str, key: &str, active: bool, dark: bool) -> El {
    el().key(key)
        .h(d::CONTROL)
        .px(d::S3)
        .items_center()
        .justify_center()
        .radius(d::R_CONTROL)
        .fill(d::themed(
            if active { d::ACCENT } else { d::ACCENT_SOFT },
            dark,
        ))
        .hover(d::themed(d::BORDER, dark))
        .child(
            text(label)
                .font(d::T_CONTROL)
                .color(d::themed(d::INK, dark)),
        )
}
pub(crate) fn view(s: &State) -> El {
    if !s.export.open {
        return el().size(0.0, 0.0);
    }
    let dark = s.editor.settings().dark;
    let mut scales = row().gap(d::S1);
    for scale in 1..=3 {
        scales = scales.child(
            button(
                &format!("{scale}×"),
                &format!("export-scale-{scale}"),
                s.export.scale == scale,
                dark,
            )
            .on_press(move |s: &mut State| s.export.scale = scale),
        );
    }
    let mut choices = col().gap(d::S2);
    for (key, label, on) in [
        ("background", "Background", s.export.background),
        ("dark", "Dark mode", s.export.dark),
        ("selection", "Selection only", s.export.selection),
    ] {
        choices = choices.child(
            button(
                &format!("{}  {label}", if on { "✓" } else { "○" }),
                &format!("export-{key}"),
                on,
                dark,
            )
            .on_press(move |s: &mut State| match key {
                "background" => s.export.background = !s.export.background,
                "dark" => s.export.dark = !s.export.dark,
                _ => s.export.selection = !s.export.selection,
            }),
        );
    }
    el().key("export-overlay")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .fill(d::themed(d::SELECTION_WASH, dark))
        .child(
            d::surface(col(), d::R_ISLAND, dark)
                .w(d::CONFIRM_WIDTH)
                .padding(d::S4)
                .gap(d::S3)
                .modal(false, |s: &mut State| s.export.open = false)
                .child(
                    text("Export drawing")
                        .font(d::T_LABEL)
                        .color(d::themed(d::INK, dark)),
                )
                .child(scales)
                .child(choices)
                .child(
                    row()
                        .gap(d::S2)
                        .child(
                            button("Save PNG", "export-png", true, dark)
                                .on_press_tx(|s: &mut State, tx| save(s, false, tx)),
                        )
                        .child(
                            button("Save SVG", "export-svg", false, dark)
                                .on_press_tx(|s: &mut State, tx| save(s, true, tx)),
                        ),
                )
                .child(
                    row()
                        .gap(d::S2)
                        .child(
                            button("Copy PNG", "copy-png", false, dark)
                                .help("Copy selection · Ctrl+Shift+C")
                                .on_press_tx(|s: &mut State, tx| copy(s, false, tx)),
                        )
                        .child(
                            button("Copy SVG", "copy-svg", false, dark)
                                .help("Copy selection · Ctrl+Alt+Shift+C")
                                .on_press_tx(|s: &mut State, tx| copy(s, true, tx)),
                        ),
                )
                .child(
                    button("Close", "close-export", false, dark)
                        .on_press(|s: &mut State| s.export.open = false),
                ),
        )
}
