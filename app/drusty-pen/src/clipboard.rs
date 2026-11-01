//! Rune carries MIME-tagged scene JSON in HTML alongside readable plain text. Its pinned
//! clipboard API has no arbitrary MIME slots, so `<pre>` is the lossless rich transport.
use crate::{State, text};
use pen_core::{Element, Kind, WorldPt, WorldRect, file};
use pen_editor::{Action, InputEvent};
use rune::{ClipboardContent, Tx};

const MIME: &str = "application/vnd.drusty-pen.elements+json";

fn encode(
    elements: &[Element],
    files: &std::collections::BTreeMap<pen_core::FileId, pen_core::BinaryFile>,
) -> Option<ClipboardContent> {
    let plain = elements
        .iter()
        .filter_map(|e| match &e.kind {
            Kind::Text { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let scene = file::SceneFile {
        dropped_bindings: 0,
        skipped_elements: 0,
        files: elements
            .iter()
            .filter_map(|e| {
                if let Kind::Image { file_id, .. } = &e.kind {
                    files.get(file_id).map(|f| (file_id.clone(), f.clone()))
                } else {
                    None
                }
            })
            .collect(),
        format: "drusty-pen".into(),
        version: file::VERSION,
        elements: elements.to_vec(),
        app_state: file::AppState::default(),
    };
    let bytes = file::save(&scene).ok()?;
    let json = String::from_utf8(bytes).ok()?;
    let escaped = json
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let content = ClipboardContent::new(plain).with_html(format!("<pre>{MIME}\n{escaped}</pre>"));
    content.html().is_some().then_some(content)
}

fn decode(content: &ClipboardContent) -> Option<file::SceneFile> {
    let html = content.html()?;
    let (_, data) = html.split_once(MIME)?;
    let (data, _) = data.split_once("</pre>")?;
    let json = data
        .trim()
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    file::load(json.as_bytes()).ok()
}

impl State {
    pub(crate) fn run_action(&mut self, action: Action, tx: &mut Tx) {
        if !action.enabled(&self.editor) {
            return;
        }
        match action {
            Action::Copy | Action::Cut => {
                if let Some(content) =
                    encode(&self.editor.copy_elements(), self.editor.document().files())
                {
                    tx.copy_rich(content);
                    self.input(InputEvent::Action(action), tx);
                } else {
                    self.files
                        .status
                        .set("Selection is too large for the clipboard");
                }
            }
            Action::Paste => {
                self.context = None;
                tx.read_clipboard(|s: &mut State, content, tx| s.paste(content, tx));
            }
            _ => {
                self.input(InputEvent::Action(action), tx);
            }
        }
    }

    pub(crate) fn paste(&mut self, content: &ClipboardContent, tx: &mut Tx) {
        if let Some(elements) = decode(content) {
            self.input(InputEvent::PasteScene(elements), tx);
        } else if let Some(image) = content.image() {
            crate::images::insert(
                self,
                image.png().to_vec(),
                pen_editor::ScreenPt::new(
                    self.editor.viewport().dx / 2.0,
                    self.editor.viewport().dy / 2.0,
                ),
                tx,
            );
        } else if !content.text().is_empty() {
            let (font, size, align) = self.editor.text_defaults();
            let (w, h) = text::measure(&mut self.fonts, content.text(), font, size, None);
            let at = self.editor.camera().screen_to_world(
                self.context
                    .unwrap_or(pen_editor::ScreenPt::new(512.0, 360.0)),
            );
            let element = Element::new(
                self.editor.document().draft_id(),
                Kind::Text {
                    content: content.text().into(),
                    font,
                    size,
                    align,
                    bounds: WorldRect::from_corners(at, WorldPt::new(at.x + w, at.y + h)),
                    container: None,
                },
                self.editor.style(),
            );
            self.input(InputEvent::Paste(vec![element]), tx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{FRAME, app, drag};
    use rune::{Key, KeyEvent, Mods};

    fn chord(rt: &mut rune::Runtime<State>, c: char) {
        rt.key(KeyEvent {
            key: Key::Command(c),
            mods: Mods {
                cmd: true,
                ..Default::default()
            },
        });
        rt.frame(FRAME);
        rt.frame(FRAME);
    }

    #[test]
    fn rich_clipboard_round_trips_through_rune_and_cut_is_one_undo() {
        let mut rt = app();
        let clipboard = rune::clipboard::memory();
        rt.use_clipboard(&clipboard);
        drag(&mut rt, (310.0, 250.0), (480.0, 390.0));
        chord(&mut rt, 'c');
        assert_eq!(rt.state.editor.selection().len(), 1);
        let content = clipboard.content().expect("copy reached Rune clipboard");
        assert!(decode(&content).is_some(), "HTML: {:?}", content.html());
        chord(&mut rt, 'v');
        assert_eq!(rt.state.editor.document().elements().len(), 2);
        chord(&mut rt, 'x');
        assert_eq!(rt.state.editor.document().elements().len(), 1);
        chord(&mut rt, 'z');
        assert_eq!(rt.state.editor.document().elements().len(), 2);
    }

    #[test]
    fn external_plain_text_pastes_as_measured_text() {
        let mut rt = app();
        let content = ClipboardContent::new("Hello\nwhiteboard");
        let mut tx = Tx::default();
        rt.state.paste(&content, &mut tx);
        let e = &rt.state.editor.document().elements()[0];
        assert!(matches!(&e.kind, Kind::Text { content, .. } if content == "Hello\nwhiteboard"));
        assert!(e.kind.bounds().width() > 30.0);
    }
    #[test]
    fn external_rich_clipboard_is_sanitized_without_losing_scene_json_or_plain_text() {
        let mut rt = app();
        let clipboard = rune::clipboard::memory();
        rt.use_clipboard(&clipboard);
        clipboard.set_text("<hello> & world");
        chord(&mut rt, 'v');
        assert!(
            matches!(&rt.state.editor.document().elements()[0].kind, Kind::Text { content, .. } if content == "<hello> & world")
        );
        chord(&mut rt, 'c');
        let copied = clipboard.content().unwrap();
        assert_eq!(copied.text(), "<hello> & world");
        clipboard.set_content(copied);
        chord(&mut rt, 'v');
        assert_eq!(rt.state.editor.document().elements().len(), 2);
        assert!(rt.state.editor.document().elements().iter().all(
            |e| matches!(&e.kind, Kind::Text { content, .. } if content == "<hello> & world")
        ));
    }
    #[test]
    fn paste_styles_measures_text_and_undoes_once() {
        let mut rt = app();
        let clipboard = rune::clipboard::memory();
        rt.use_clipboard(&clipboard);
        clipboard.set_text("Source");
        chord(&mut rt, 'v');
        rt.update(|s, tx| {
            s.input(
                InputEvent::TextStyle {
                    font: pen_core::Font::Code,
                    size: 36.0,
                    align: pen_core::TextAlign::Right,
                },
                tx,
            );
        });
        rt.frame(FRAME);
        rt.key(KeyEvent {
            key: Key::Command('c'),
            mods: Mods {
                cmd: true,
                alt: true,
                ..Default::default()
            },
        });
        rt.frame(FRAME);
        clipboard.set_text("Different text");
        chord(&mut rt, 'v');
        rt.update(|s, tx| {
            s.input(
                InputEvent::TextStyle {
                    font: pen_core::Font::Normal,
                    size: 16.0,
                    align: pen_core::TextAlign::Left,
                },
                tx,
            );
        });
        rt.frame(FRAME);
        rt.update(|s, tx| {
            s.input(
                InputEvent::Property(pen_core::Property::Stroke(pen_core::Srgba::rgb(0xff0000))),
                tx,
            );
        });
        rt.frame(FRAME);
        let id = rt.state.editor.selection().ids().next().unwrap();
        let before = rt.state.editor.document().get(id).unwrap().kind.clone();
        rt.key(KeyEvent {
            key: Key::Command('v'),
            mods: Mods {
                cmd: true,
                alt: true,
                ..Default::default()
            },
        });
        rt.frame(FRAME);
        let after = rt.state.editor.document().get(id).unwrap();
        assert!(
            matches!(after.kind, Kind::Text { font: pen_core::Font::Code, size, align: pen_core::TextAlign::Right, .. } if (size - 36.0).abs() < 0.01)
        );
        assert!(after.kind.bounds().width() > before.bounds().width() * 1.5);
        assert_eq!(rt.state.hex[0].text, "#1e1e1e");
        chord(&mut rt, 'z');
        assert_eq!(rt.state.editor.document().get(id).unwrap().kind, before);
    }
}
