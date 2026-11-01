use super::*;
use crate::tests::{FRAME, app, drag, key};
use pen_core::Kind;
use rune::{ClipboardContent, ClipboardImage, FileDialogKind, KeyEvent, Mods};
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/pen-core/tests/fixtures/landscape.png")
}
fn click(rt: &mut rune::Runtime<State>, name: &str) {
    let (x, y) = rt.find(name).unwrap_or_else(|| panic!("missing {name}"));
    drag(rt, (x, y), (x, y));
}
fn chord(rt: &mut rune::Runtime<State>, c: char, shift: bool) {
    rt.key(KeyEvent {
        key: Key::Command(c),
        mods: Mods {
            cmd: true,
            shift,
            ..Mods::default()
        },
    });
    rt.frame(FRAME);
    rt.frame(FRAME);
}
#[test]
fn image_tool_drop_crop_clipboard_and_export_use_real_rune_input() {
    let mut rt = app();
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    key(&mut rt, "9");
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Open);
    rt.resolve_file_dialog(Some(fixture())).unwrap();
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.document().files().len(), 1);
    assert!(matches!(
        rt.state.editor.document().elements()[0].kind,
        Kind::Image { .. }
    ));
    let original = rt.state.editor.document().elements()[0].kind.clone();
    click(&mut rt, "crop-left");
    assert!(
        rt.state.editor.document().elements()[0]
            .kind
            .bounds()
            .width()
            < original.bounds().width()
    );
    chord(&mut rt, 'z', false);
    assert_eq!(rt.state.editor.document().elements()[0].kind, original);
    chord(&mut rt, 'c', false);
    chord(&mut rt, 'v', false);
    assert_eq!(rt.state.editor.document().elements().len(), 2);
    assert_eq!(rt.state.editor.document().files().len(), 1);
    rt.drop_files(vec![fixture()], Some((650.0, 400.0)));
    rt.frame(FRAME);
    assert_eq!(rt.state.editor.document().elements().len(), 3);
    chord(&mut rt, 'e', true);
    assert!(rt.find("export-png").is_some());
    click(&mut rt, "copy-png");
    rt.frame(FRAME);
    let copied = clipboard.content().unwrap();
    assert!(copied.image().is_some());
    click(&mut rt, "copy-svg");
    rt.frame(FRAME);
    assert!(clipboard.content().unwrap().text().contains("<svg"));
    click(&mut rt, "export-scale-2");
    click(&mut rt, "export-png");
    assert_eq!(rt.file_dialog().unwrap().0, FileDialogKind::Save);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.png");
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(FRAME);
    assert!(image::open(path).unwrap().width() > 720);
}
#[test]
fn external_image_paste_and_copy_to_another_document_keep_embedded_bytes() {
    let mut rt = app();
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    let bytes = std::fs::read(fixture()).unwrap();
    clipboard.set_content(
        ClipboardContent::new("").with_image(ClipboardImage::from_png(bytes).unwrap()),
    );
    chord(&mut rt, 'v', false);
    chord(&mut rt, 'c', false);
    let mut other = app();
    other.use_clipboard(&clipboard);
    chord(&mut other, 'v', false);
    assert_eq!(other.state.editor.document().files().len(), 1);
    assert_eq!(other.state.editor.document().elements().len(), 1);
    chord(&mut other, 'z', false);
    assert!(other.state.editor.document().files().is_empty());
    assert!(other.state.editor.document().elements().is_empty());
}
#[test]
fn image_scene_pan_and_zoom_reuse_decoded_paint() {
    let mut rt = app();
    rt.drop_files(vec![fixture()], Some((400.0, 240.0)));
    rt.frame(FRAME);
    let commits = rt.stats.commits;
    rt.wheel(500.0, 350.0, 14.0);
    rt.frame(FRAME);
    assert_eq!(rt.stats.commits, commits);
    assert!(rt.state.snapshot().document().is_ok());
}
