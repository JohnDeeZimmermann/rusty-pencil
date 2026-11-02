//! Browser backend. App data lives in IndexedDB, loaded into memory once before the app starts so
//! every read stays synchronous; writes update memory at once and reach IndexedDB in order. User
//! files come from `<input type="file">` and drops, and leave as downloads.
//!
//! DOM callbacks reach the app through its `AppHandle`, which runs them on the next turn of the
//! event loop with the app state.
use super::Picked;
use crate::State;
use js_sys::{Array, Function, Promise, Reflect, Uint8Array};
use rune::{App, AppHandle, FileDialogOptions, Tx};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// Verb for a finished save, as the status line shows it.
pub(crate) const SAVED: &str = "Downloaded";

/// The root of every app-data key.
const DATA_DIR: &str = "/rusty-pencil";

/// A small IndexedDB wrapper: one object store of byte arrays keyed by path. Transactions on one
/// store commit in the order they were created, so the last write wins.
const STORE_JS: &str = r"
const request = (r) => new Promise((ok, fail) => { r.onsuccess = () => ok(r.result); r.onerror = () => fail(r.error); });
let db = null;
const store = (mode) => db.transaction('data', mode).objectStore('data');
return {
    async load() {
        const open = indexedDB.open('rusty-pencil', 1);
        open.onupgradeneeded = () => open.result.createObjectStore('data');
        db = await request(open);
        const s = store('readonly');
        const [keys, values] = await Promise.all([request(s.getAllKeys()), request(s.getAll())]);
        return keys.map((key, i) => [key, values[i]]);
    },
    // Commit at once rather than when the task ends, so a write made while the page hides
    // (`pagehide`) lands before the page unloads.
    put(key, bytes) { if (db) { const s = store('readwrite'); s.put(bytes, key); s.transaction.commit?.(); } },
    remove(key) { if (db) { const s = store('readwrite'); s.delete(key); s.transaction.commit?.(); } },
};
";

thread_local! {
    /// The in-memory mirror of IndexedDB, keyed by path. Not app state: the platform's cache of
    /// what the browser holds, filled once before the app starts.
    static DATA: RefCell<BTreeMap<String, Vec<u8>>> = const { RefCell::new(BTreeMap::new()) };
    /// The IndexedDB wrapper, or `None` when the browser offers no storage (the app then runs as
    /// an unsaved scratch canvas).
    static STORE: RefCell<Option<JsValue>> = const { RefCell::new(None) };
    static HANDLE: RefCell<Option<AppHandle<State>>> = const { RefCell::new(None) };
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn call(method: &str, args: &[JsValue]) {
    STORE.with_borrow(|store| {
        let Some(store) = store else { return };
        if let Ok(f) = Reflect::get(store, &method.into()).and_then(JsCast::dyn_into::<Function>) {
            let _ = f.apply(store, &args.iter().collect::<Array>());
        }
    });
}

pub(crate) fn data_dir() -> Option<PathBuf> {
    STORE
        .with_borrow(Option::is_some)
        .then(|| PathBuf::from(DATA_DIR))
}

pub(crate) fn read(path: &Path) -> Result<Vec<u8>, String> {
    DATA.with_borrow(|data| data.get(&key(path)).cloned())
        .ok_or_else(|| format!("{} is not stored", path.display()))
}

pub(crate) fn exists(path: &Path) -> bool {
    DATA.with_borrow(|data| data.contains_key(&key(path)))
}

pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if STORE.with_borrow(Option::is_none) {
        return Err("this browser offers no local storage".into());
    }
    let key = key(path);
    call("put", &[key.clone().into(), Uint8Array::from(bytes).into()]);
    DATA.with_borrow_mut(|data| data.insert(key, bytes.to_vec()));
    Ok(())
}

pub(crate) fn remove(path: &Path) -> Result<(), String> {
    let key = key(path);
    call("remove", &[key.clone().into()]);
    DATA.with_borrow_mut(|data| data.remove(&key));
    Ok(())
}

/// Saves a user file as a download named after the path's file name.
pub(crate) fn deliver(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let error = |e: JsValue| format!("{e:?}");
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let parts = Array::of1(&Uint8Array::from(bytes));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let blob =
        web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(error)?;
    let link: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(error)?
        .unchecked_into();
    link.set_href(&url);
    link.set_download(&crate::files::name(path));
    link.click();
    // Firefox starts the download after the click returns; revoke the URL a little later.
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    let _ = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 10_000);
    Ok(())
}

/// Browser files carry only a name, so two of them are never known to be the same file.
pub(crate) fn same_file(_: &Path, _: &Path) -> bool {
    false
}

pub(crate) fn file_identity(path: &Path) -> PathBuf {
    PathBuf::from(crate::files::name(path))
}

/// Browser pickers give names, not reusable disk access. Keep a bounded local scene copy.
pub(crate) fn cache_recent(
    directory: &Path,
    path: &Path,
    scene: &pen_core::file::SceneFile,
) -> Result<(), String> {
    let bytes = pen_core::file::save(scene).map_err(|e| e.to_string())?;
    write(&recent_source(directory, path), &bytes)
}

pub(crate) fn recent_source(directory: &Path, path: &Path) -> PathBuf {
    directory.join("recent").join(crate::files::name(path))
}

pub(crate) fn remove_recent_copy(directory: &Path, path: &Path) -> Result<(), String> {
    remove(&recent_source(directory, path))
}

pub(crate) fn recent_location(_: &Path) -> String {
    "Saved in this browser".into()
}

/// Runs `f` with the app state on the next turn of the event loop.
fn update(f: impl FnOnce(&mut State, &mut Tx) + Send + 'static) {
    HANDLE.with_borrow(|handle| {
        if let Some(handle) = handle {
            handle.update(f);
        }
    });
}

async fn bytes(file: web_sys::File) -> Result<Picked, String> {
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(Picked {
        path: PathBuf::from(file.name()),
        bytes: Uint8Array::new(&buffer).to_vec(),
    })
}

/// Opens the browser's file picker with the dialog's filters; `then` runs once a file is read.
/// A cancelled picker never calls `then`.
pub(crate) fn open_file(
    _: &mut Tx,
    options: FileDialogOptions,
    then: impl FnOnce(&mut State, Result<Option<Picked>, String>, &mut Tx) + Send + 'static,
) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(input) = document
        .create_element("input")
        .map(JsCast::unchecked_into::<web_sys::HtmlInputElement>)
    else {
        return;
    };
    input.set_type("file");
    let FileDialogOptions { filters, .. } = options;
    let accept: Vec<String> = filters
        .iter()
        .flat_map(|(_, extensions)| extensions.iter().map(|e| format!(".{e}")))
        .collect();
    input.set_accept(&accept.join(","));
    let picker = input.clone();
    let changed = Closure::once_into_js(move || {
        let Some(file) = picker.files().and_then(|files| files.get(0)) else {
            return;
        };
        spawn_local(async move {
            let picked = bytes(file).await.map(Some);
            update(move |s, tx| then(s, picked, tx));
        });
    });
    input.set_onchange(Some(changed.unchecked_ref()));
    input.click();
}

/// The browser has no save dialog: the answer is the suggested file name, which [`deliver`]
/// downloads under.
pub(crate) fn save_file(
    s: &mut State,
    tx: &mut Tx,
    options: FileDialogOptions,
    then: impl FnOnce(&mut State, Result<Option<PathBuf>, String>, &mut Tx) + Send + 'static,
) {
    let name = options.file_name.unwrap_or_else(|| "Untitled".into());
    then(s, Ok(Some(PathBuf::from(name))), tx);
}

/// Loads stored app data, wires the page's lifecycle and drop events to the app, then runs it.
pub(crate) fn run(app: App<State>) {
    HANDLE.with_borrow_mut(|handle| *handle = Some(app.handle()));
    listen();
    spawn_local(async move {
        if let Err(error) = load().await {
            web_sys::console::warn_1(&format!("Running without local storage: {error}").into());
        }
        app.run();
    });
}

async fn load() -> Result<(), String> {
    let error = |e: JsValue| format!("{e:?}");
    let store = Function::new_no_args(STORE_JS)
        .call0(&JsValue::NULL)
        .map_err(error)?;
    let load: Function = Reflect::get(&store, &"load".into())
        .map_err(error)?
        .dyn_into()
        .map_err(error)?;
    let promise: Promise = load
        .call0(&store)
        .map_err(error)?
        .dyn_into()
        .map_err(error)?;
    let entries: Array = JsFuture::from(promise)
        .await
        .map_err(error)?
        .dyn_into()
        .map_err(error)?;
    DATA.with_borrow_mut(|data| {
        for entry in entries.iter() {
            let entry = Array::from(&entry);
            if let Some(key) = entry.get(0).as_string() {
                data.insert(key, Uint8Array::new(&entry.get(1)).to_vec());
            }
        }
    });
    STORE.with_borrow_mut(|s| *s = Some(store));
    Ok(())
}

fn on(target: &web_sys::EventTarget, event: &str, f: impl FnMut(web_sys::Event) + 'static) {
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(f);
    let _ = target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
    // Listeners live as long as the page.
    closure.forget();
}

fn listen() {
    let Some(window) = web_sys::window() else {
        return;
    };
    // Closing or hiding the page flushes pending edits, as closing the window does on desktop.
    on(&window, "pagehide", |_| update(crate::files::close));
    if let Some(document) = window.document() {
        let hidden = document.clone();
        on(&document, "visibilitychange", move |_| {
            if hidden.hidden() {
                update(crate::files::close);
            }
        });
    }
    on(&window, "dragover", |event| event.prevent_default());
    on(&window, "drop", |event| {
        event.prevent_default();
        let Some(event) = event.dyn_ref::<web_sys::DragEvent>() else {
            return;
        };
        let at = pen_editor::ScreenPt::new(event.client_x() as f32, event.client_y() as f32);
        let Some(list) = event.data_transfer().and_then(|d| d.files()) else {
            return;
        };
        let files: Vec<web_sys::File> = (0..list.length()).filter_map(|i| list.get(i)).collect();
        spawn_local(async move {
            let mut picked = Vec::with_capacity(files.len());
            for file in files {
                match bytes(file).await {
                    Ok(file) => picked.push(file),
                    Err(error) => web_sys::console::warn_1(&error.into()),
                }
            }
            update(move |s, tx| crate::images::drop_picked(s, picked, at, tx));
        });
    });
}
