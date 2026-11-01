//! Personal, offline scene snippets; fresh identities and references are assigned on drop.
use crate::{State, design as d, polish::button};
use pen_core::{WorldRect, file::SceneFile};
use pen_editor::{InputEvent, ScreenPt};
use rune::{El, FileDialogOptions, Phase, Tx, col, el, row, text};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Default)]
pub(crate) struct Library {
    pub open: bool,
    revision: u64,
    pub items: Vec<SceneFile>,
    path: Option<PathBuf>,
    rects: BTreeMap<usize, [f32; 4]>,
    pub dragging: Option<(usize, ScreenPt)>,
}
#[derive(Serialize, Deserialize)]
struct File {
    format: String,
    version: u32,
    items: Vec<SceneFile>,
}
fn encode(items: &[SceneFile]) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(&File {
        format: "drusty-pen-library".into(),
        version: 1,
        items: items.to_vec(),
    })
    .map_err(|e| e.to_string())
}
fn decode(bytes: &[u8]) -> Result<Vec<SceneFile>, String> {
    let file: File = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if file.format != "drusty-pen-library" || file.version != 1 {
        return Err("Unsupported library file".into());
    }
    for item in &file.items {
        item.document().map_err(|e| e.to_string())?;
    }
    Ok(file.items)
}
pub(crate) fn restore(s: &mut State) {
    s.library.path = s
        .tabs
        .workspace
        .as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.join("library.drusty-library"));
    if let Some(path) = s.library.path.as_ref().filter(|p| p.exists()) {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| decode(&bytes))
        {
            Ok(items) => {
                s.library.items = items;
                s.library.revision = s.library.revision.wrapping_add(1);
            }
            Err(e) => s
                .files
                .status
                .set(format!("Could not restore library: {e}")),
        }
    }
}
fn persist(s: &mut State) {
    s.library.revision = s.library.revision.wrapping_add(1);
    if let Some(path) = &s.library.path
        && let Err(e) =
            encode(&s.library.items).and_then(|bytes| crate::files::atomic_write(path, &bytes))
    {
        s.files.status.set(format!("Could not save library: {e}"));
    }
}
pub(crate) fn add(s: &mut State) {
    let elements = s.editor.copy_elements();
    if elements.is_empty() {
        s.files
            .status
            .set("Select something to add to your library");
        return;
    }
    let mut scene = SceneFile::new(s.editor.document(), Default::default());
    scene.elements = elements;
    // Standalone items retain only local references and their actual embedded assets.
    let ids: Vec<_> = scene.elements.iter().map(pen_core::Element::id).collect();
    for e in &mut scene.elements {
        e.frame_id = e.frame_id.filter(|id| ids.contains(id));
        for b in &mut e.bindings {
            *b = b.filter(|b| ids.contains(&b.element));
        }
        if matches!(e.link,Some(pen_core::Link::Element(id)) if !ids.contains(&id)) {
            e.link = None;
        }
    }
    scene.files.retain(|id, _| {
        scene
            .elements
            .iter()
            .any(|e| matches!(&e.kind,pen_core::Kind::Image { file_id,.. } if file_id==id))
    });
    s.library.items.push(scene);
    persist(s);
    s.files.status.set("Added selection to library");
}
fn import(_: &mut State, tx: &mut Tx) {
    tx.open_file(
        FileDialogOptions {
            title: "Import library".into(),
            ..Default::default()
        }
        .filter("drusty-pen library", &["drusty-library", "json"]),
        |s: &mut State, result, _| match result {
            Ok(Some(path)) => match std::fs::read(path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| decode(&bytes))
            {
                Ok(items) => {
                    for item in items {
                        if !s.library.items.contains(&item) {
                            s.library.items.push(item);
                        }
                    }
                    persist(s);
                    s.files.status.set("Imported library");
                }
                Err(e) => s.files.status.set(format!("Could not import library: {e}")),
            },
            Ok(None) => {}
            Err(e) => s.files.status.set(e.to_string()),
        },
    );
}
fn export(s: &mut State, tx: &mut Tx) {
    let bytes = match encode(&s.library.items) {
        Ok(bytes) => bytes,
        Err(e) => {
            s.files.status.set(e);
            return;
        }
    };
    tx.save_file(
        FileDialogOptions {
            title: "Export library".into(),
            file_name: Some("Library.drusty-library".into()),
            ..Default::default()
        }
        .filter("drusty-pen library", &["drusty-library"]),
        move |s: &mut State, result, _| match result {
            Ok(Some(path)) => {
                if let Err(e) = crate::files::atomic_write(&path, &bytes) {
                    s.files.status.set(e);
                } else {
                    s.files.status.set("Exported library");
                }
            }
            Ok(None) => {}
            Err(e) => s.files.status.set(e.to_string()),
        },
    );
}
fn thumbnail(scene: &SceneFile, dark: bool) -> El {
    let Ok(doc) = scene.document() else {
        return el();
    };
    let Some(bounds) = scene
        .elements
        .iter()
        .map(|e| {
            let b = e.bounds();
            if matches!(e.kind, pen_core::Kind::Frame { .. }) {
                WorldRect::from_corners(
                    pen_core::WorldPt::new(b.min().x, b.min().y - 24.0),
                    b.max(),
                )
            } else {
                b
            }
        })
        .reduce(WorldRect::union)
    else {
        return el();
    };
    let mut list = pen_render::DrawList::new();
    for e in doc.elements() {
        pen_render::frames::draw(&doc, &[], e, &mut list);
    }
    let mut images = crate::images::Cache::default();
    images.update(&doc);
    let available_w = d::LIBRARY_WIDTH - d::S3 * 2.0;
    let scale = ((available_w - d::S2 * 2.0) / bounds.width().max(1.0))
        .min((d::LIBRARY_THUMB - d::S2 * 2.0) / bounds.height().max(1.0))
        .min(1.0);
    el().h(d::LIBRARY_THUMB)
        .w_full()
        .inert()
        .clip_rounded(d::R_CONTROL)
        .child(
            el().absolute(
                (available_w - bounds.width() * scale) / 2.0,
                (d::LIBRARY_THUMB - bounds.height() * scale) / 2.0,
            )
            .size(0.0, 0.0)
            .scale(scale)
            .origin(0.0, 0.0)
            .child(
                el().absolute(-bounds.min().x, -bounds.min().y)
                    .size(0.0, 0.0)
                    .child(crate::text::paint_with_images(
                        list.prims(),
                        pen_core::Settings {
                            dark,
                            ..Default::default()
                        },
                        &images,
                    )),
            ),
        )
}
pub(crate) fn view(s: &State) -> El {
    if !s.library.open || s.editor.modes().zen {
        return el().size(0.0, 0.0);
    }
    let dark = s.editor.settings().dark;
    let mut items = col().gap(d::S2);
    for (i, item) in s.library.items.iter().enumerate() {
        items = items.child(
            d::surface(col(), d::R_CONTROL, dark)
                .key(format!("library-item-{i}"))
                .child(rune::memo(
                    format!("library-thumb-{i}"),
                    (s.library.revision, dark),
                    || thumbnail(item, dark),
                ))
                .on_layout_tx(move |s: &mut State, rect, tx| {
                    s.library.rects.insert(i, rect);
                    tx.live_only();
                })
                .on_drag(move |s: &mut State, drag, tx| {
                    let Some(rect) = s.library.rects.get(&i).copied() else {
                        return;
                    };
                    let at = ScreenPt::new(rect[0] + drag.x, rect[1] + drag.y);
                    if drag.phase == Phase::Up {
                        s.library.dragging = None;
                        if drag.cancelled() || s.editor.modes().view {
                            return;
                        }
                        if drag.dx.hypot(drag.dy) < 4.0 {
                            let at = ScreenPt::new(
                                s.editor.viewport().dx / 2.0,
                                s.editor.viewport().dy / 2.0,
                            );
                            if let Some(scene) = s.library.items.get(i).cloned() {
                                s.input(InputEvent::PlaceScene { scene, at }, tx);
                            }
                        } else if at.x < s.editor.viewport().dx - d::LIBRARY_WIDTH - d::S4
                            && let Some(scene) = s.library.items.get(i).cloned()
                        {
                            s.input(InputEvent::PlaceScene { scene, at }, tx);
                        }
                    } else {
                        s.library.dragging = Some((i, at));
                    }
                })
                .child(
                    button("Remove", &format!("library-remove-{i}"), false, dark).on_press(
                        move |s: &mut State| {
                            s.library.items.remove(i);
                            persist(s);
                        },
                    ),
                ),
        );
    }
    if s.library.items.is_empty() {
        items = items.child(
            text("Keep shapes you use often.\nSelect elements, then add them here.")
                .font(d::T_HINT)
                .color(d::themed(d::MUTED, dark)),
        );
    }
    d::surface(col().gap(d::S3), d::R_ISLAND, dark)
        .key("personal-library")
        .instant_changes()
        .w(d::LIBRARY_WIDTH)
        .padding(d::S3)
        .pin(None, Some(d::DETAILS_TOP), Some(d::S4), None)
        .on_drag(|_: &mut State, _, tx| tx.live_only())
        .child(
            text("Personal library")
                .font(d::T_LABEL)
                .color(d::themed(d::INK, dark)),
        )
        .child(
            button("+ Add selection", "library-add", true, dark).on_press(|s: &mut State| add(s)),
        )
        .child(
            row()
                .gap(d::S2)
                .child(button("Import", "library-import", false, dark).on_press_tx(import))
                .child(button("Export", "library-export", false, dark).on_press_tx(export)),
        )
        .child(
            items
                .max_h(
                    (s.editor.viewport().dy - d::DETAILS_TOP - d::S4 - d::CONTROL * 4.0)
                        .max(d::LIBRARY_THUMB),
                )
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                }),
        )
}
pub(crate) fn ghost(s: &State) -> El {
    let Some((i, at)) = s.library.dragging else {
        return el().size(0.0, 0.0);
    };
    let Some(item) = s.library.items.get(i) else {
        return el().size(0.0, 0.0);
    };
    el().key("library-ghost")
        .absolute(at.x - d::LIBRARY_WIDTH / 2.0, at.y - d::LIBRARY_THUMB / 2.0)
        .w(d::LIBRARY_WIDTH)
        .opacity(0.6)
        .inert()
        .child(rune::memo(
            "library-ghost-paint",
            (s.library.revision, i, s.editor.settings().dark),
            || thumbnail(item, s.editor.settings().dark),
        ))
}
