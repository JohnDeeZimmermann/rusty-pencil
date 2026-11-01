//! Bounded decoding and native image entry points. Camera changes never decode images.
use crate::State;
use pen_core::{BinaryFile, Document, FileId};
use pen_editor::{InputEvent, ScreenPt};
use rune::{El, FileDialogOptions, Pixels, Tx, el};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) const MAX_ENTRIES: usize = 64;
pub(crate) const MAX_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Debug, Default)]
pub(crate) struct Cache {
    entries: BTreeMap<FileId, Pixels>,
}
impl Cache {
    pub(crate) fn update(&mut self, doc: &Document) {
        let active: std::collections::BTreeSet<_> = doc
            .elements()
            .iter()
            .filter_map(|e| {
                if let pen_core::Kind::Image { file_id, .. } = &e.kind {
                    Some(file_id)
                } else {
                    None
                }
            })
            .collect();
        let mut bytes = 0;
        let mut count = 0;
        let allowed: std::collections::BTreeSet<_> = doc
            .files()
            .iter()
            .filter_map(|(id, file)| {
                let size = u64::from(file.width) * u64::from(file.height) * 4;
                if !active.contains(id) || count >= MAX_ENTRIES || bytes + size > MAX_BYTES {
                    return None;
                }
                bytes += size;
                count += 1;
                Some(id)
            })
            .collect();
        // Evict before allocating replacements, so admission never temporarily exceeds the cap.
        self.entries.retain(|id, _| allowed.contains(id));
        for id in allowed {
            let Some(file) = doc.files().get(id) else {
                continue;
            };
            if !self.entries.contains_key(id)
                && let Ok(image) = image::load_from_memory(&file.data)
            {
                let pixels = Pixels::new();
                pixels.set(file.width, file.height, image.into_rgba8().into_raw());
                self.entries.insert(id.clone(), pixels);
            }
        }
    }
    pub(crate) fn view(&self, prim: &pen_render::Prim) -> El {
        let pen_render::Prim::Image {
            file_id,
            crop,
            bounds: b,
            angle,
            opacity,
        } = prim
        else {
            return el();
        };
        let node = el()
            .absolute(b.min().x, b.min().y)
            .size(b.width(), b.height())
            .rotate(*angle)
            .opacity(f32::from(*opacity) / 100.0)
            .instant_changes();
        let Some(pixels) = self.entries.get(file_id) else {
            return node
                .fill(crate::design::c(pen_core::Srgba::rgb(0xe5e8e5)))
                .child(rune::text("Image unavailable").font(crate::design::T_HINT));
        };
        let w = b.width() / crop.width;
        let h = b.height() / crop.height;
        node.child(
            el().size(b.width(), b.height()).clip_rounded(0.0).child(
                el().absolute(-crop.x * w, -crop.y * h)
                    .size(w, h)
                    .pixels(pixels),
            ),
        )
    }
}
fn centre(s: &State) -> ScreenPt {
    ScreenPt::new(s.editor.viewport().dx / 2.0, s.editor.viewport().dy / 2.0)
}
pub(crate) fn insert(s: &mut State, bytes: Vec<u8>, at: ScreenPt, tx: &mut Tx) {
    match BinaryFile::from_bytes(bytes) {
        Some(file) => {
            if image::load_from_memory(&file.data).is_err() {
                s.files.status.set("Could not decode image");
                return;
            }
            let active = s.snapshot().files;
            let total: u64 = active
                .values()
                .map(|f| u64::from(f.width) * u64::from(f.height) * 4)
                .sum();
            if !s.editor.document().files().contains_key(&file.id())
                && (active.len() >= MAX_ENTRIES
                    || total + u64::from(file.width) * u64::from(file.height) * 4 > MAX_BYTES)
            {
                s.files
                    .status
                    .set("Image limit reached (64 images / 128 MiB decoded)");
                return;
            }
            s.input(InputEvent::InsertImage { file, at }, tx);
        }
        None => s
            .files
            .status
            .set("Choose a PNG or JPEG up to 16 MiB / 16 megapixels"),
    }
}
pub(crate) fn open(s: &mut State, path: &Path, at: ScreenPt, tx: &mut Tx) {
    match std::fs::read(path) {
        Ok(bytes) => insert(s, bytes, at, tx),
        Err(e) => s.files.status.set(format!("Could not insert image: {e}")),
    }
}
pub(crate) fn dialog(s: &mut State, tx: &mut Tx) {
    s.menu = None;
    crate::platform::open_file(
        tx,
        FileDialogOptions {
            title: "Insert image".into(),
            ..Default::default()
        }
        .filter("Image", &["png", "jpg", "jpeg"]),
        |s, result, tx| match result {
            Ok(Some(picked)) => {
                let at = centre(s);
                insert(s, picked.bytes, at, tx);
            }
            Ok(None) => {}
            Err(e) => s.files.status.set(format!("Could not insert image: {e}")),
        },
    );
}
fn is_scene(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("excalidraw" | "rusty-pencil" | "drusty-pen" | "json")
    )
}
/// Each further dropped image lands this far (screen points) below and right of the last.
const DROP_STEP: f32 = 20.0;
/// Files dropped by the browser, already read: scenes open in tabs, images land at `at`.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn drop_picked(
    s: &mut State,
    files: Vec<crate::platform::Picked>,
    at: ScreenPt,
    tx: &mut Tx,
) {
    for (i, file) in files.into_iter().enumerate() {
        if is_scene(&file.path) {
            s.open_picked(&file, tx);
        } else {
            let step = i as f32 * DROP_STEP;
            insert(s, file.bytes, ScreenPt::new(at.x + step, at.y + step), tx);
        }
    }
}
pub(crate) fn drop_files(s: &mut State, files: &[PathBuf], tx: &mut Tx) {
    let at = tx
        .drop_point()
        .map_or_else(|| centre(s), |p| ScreenPt::new(p.x, p.y));
    for (i, path) in files.iter().enumerate() {
        if is_scene(path) {
            s.open_path(path, tx);
        } else {
            open(
                s,
                path,
                ScreenPt::new(at.x + i as f32 * DROP_STEP, at.y + i as f32 * DROP_STEP),
                tx,
            );
        }
    }
}

pub(crate) fn crop_controls(s: &State) -> El {
    let dark = s.editor.settings().dark;
    let mut controls = rune::col().gap(crate::design::S1).child(
        rune::text("Crop image")
            .font(crate::design::T_CAPTION)
            .color(crate::design::themed(crate::design::MUTED, dark)),
    );
    for (key, label) in [
        ("left", "Trim left"),
        ("right", "Trim right"),
        ("top", "Trim top"),
        ("bottom", "Trim bottom"),
        ("reset", "Reset crop"),
    ] {
        controls = controls.child(
            el().key(format!("crop-{key}"))
                .w_full()
                .h(crate::design::CONTROL)
                .radius(crate::design::R_CONTROL)
                .hover(crate::design::themed(crate::design::ACCENT_SOFT, dark))
                .help("Trim 10% of the visible source; reset restores the original image")
                .child(
                    rune::text(label)
                        .font(crate::design::T_CONTROL)
                        .color(crate::design::themed(crate::design::INK, dark)),
                )
                .on_press_tx(move |s: &mut State, tx| {
                    let Some(mut crop) = s
                        .editor
                        .selection()
                        .ids()
                        .filter_map(|id| s.editor.document().get(id))
                        .find_map(|e| {
                            if let pen_core::Kind::Image { crop, .. } = e.kind {
                                Some(crop)
                            } else {
                                None
                            }
                        })
                    else {
                        return;
                    };
                    match key {
                        "left" => {
                            crop.x += crop.width * 0.1;
                            crop.width *= 0.9;
                        }
                        "right" => crop.width *= 0.9,
                        "top" => {
                            crop.y += crop.height * 0.1;
                            crop.height *= 0.9;
                        }
                        "bottom" => crop.height *= 0.9,
                        _ => crop = pen_core::Crop::default(),
                    }
                    if crop.width >= 0.01 && crop.height >= 0.01 {
                        s.input(InputEvent::CropImage(crop), tx);
                    }
                }),
        );
    }
    controls
}

#[cfg(test)]
#[path = "image_cache_tests.rs"]
mod tests;

/// Source-pixel sampling respects the same crop and inverse rotation as image painting.
pub(crate) fn sample(s: &State, at: ScreenPt) -> Option<pen_core::Srgba> {
    let point = s.editor.camera().screen_to_world(at);
    let e = s.editor.document().hit(point, 0.0, false, false)?;
    let pen_core::Kind::Image {
        file_id,
        crop,
        bounds,
    } = &e.kind
    else {
        return None;
    };
    let file = s.editor.document().files().get(file_id)?;
    let p = pen_core::rotate(point, bounds.centre(), -e.angle);
    let x = (crop.x + (p.x - bounds.min().x) / bounds.width().max(0.001) * crop.width)
        * file.width as f32;
    let y = (crop.y + (p.y - bounds.min().y) / bounds.height().max(0.001) * crop.height)
        * file.height as f32;
    let image = image::load_from_memory(&file.data).ok()?.into_rgba8();
    let rgba = image
        .get_pixel(
            (x as u32).min(file.width - 1),
            (y as u32).min(file.height - 1),
        )
        .0;
    Some(pen_core::Srgba {
        r: rgba[0],
        g: rgba[1],
        b: rgba[2],
        a: rgba[3],
    })
}
