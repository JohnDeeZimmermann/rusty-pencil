# M10: Run in the browser

**Goal:** the same app runs as a static WebAssembly page, with local persistence and file
open/save, without forking the code base. Native stays the primary target; nothing on native
may regress.

**Estimate:** 3–4 days. Phase 1 is a half-day spike; decide after it whether to continue.

## Done when

- [x] A web build script produces a static folder that serves from `localhost` and shows the
      canvas in a WebGPU browser, with the same visuals as native. (`app/rusty-pencil/web/build.sh`,
      which follows rune's `scripts/web-build.sh` plus a temporary rune patch; see Notes.)
- [x] Drawing, selection, pan/zoom, text, properties, tabs and undo work in the browser with
      native-equivalent smoothness.
- [x] Autosave, workspace recovery, preferences and the personal library survive a page reload.
- [x] Open a scene, `.excalidraw` or library file from disk; save scenes and export PNG/SVG and
      library files as downloads.
- [x] Drag-and-drop of images and scene files onto the canvas works.
- [x] Copy and paste (rich scene, plain text, images) work through the browser clipboard.
- [x] No shortcut depends on a key the browser reserves (Ctrl+W, Ctrl+T, Ctrl+N).
- [x] CI-style check: `cargo check -p rusty-pencil --target wasm32-unknown-unknown` passes, and
      a Playwright smoke test (`app/rusty-pencil/web/smoke.mjs`) captures screenshots of the web
      build.

## Starting point

A spike against rune rev `7919714` (no repo changes) found:

- `pen-core`, `pen-editor` and `pen-render` already compile for `wasm32-unknown-unknown`.
- The app compiles and links once `native-file-dialog` is native-only. Rune refuses that
  feature (and `hot`) on wasm.
- A release `.wasm` is about 17 MB raw and 5.9 MB gzipped before wasm-bindgen and `wasm-opt`.
  The three bundled fonts are about 1.3 MB of that.
- `std::fs` returns errors on wasm rather than panicking, and `ProjectDirs` returns `None`,
  so an unported build runs as an unsaved scratch canvas.
- Popup menus already fall back to in-app menus (`rune::popup_menu_supported()` is false).
- Rune's web clipboard supports rich HTML, text and PNG, which the app's `<pre>` transport uses.
- Rune's web shell has no file-dialog backend and no file-drop events. Its dialogs return
  `PathBuf`, which has no meaning in a browser.

Rune's `WEB.md` (in the pinned checkout) documents the build script, entry point, fonts,
clipboard rules and the Playwright proof.

## Tasks

### Phase 1: scratch canvas in the browser (spike)

1. [x] Manifest: move `native-file-dialog`, `directories` and `tempfile` under
       `cfg(not(target_arch = "wasm32"))`. On wasm, drop rune's `embedded-fonts` (the app
       supplies its own through `AppSpec::fonts`).
2. [x] Add `app/rusty-pencil/src/lib.rs` with `crate-type = ["cdylib", "rlib"]`; `main.rs`
       and a `#[wasm_bindgen(start)]` function both call one shared `run()`.
3. [x] Build with rune's `scripts/web-build.sh rusty-pencil` and check the page in Chromium.
       Record frame smoothness for draw, drag and pan/zoom against native.
4. [x] Decide go/no-go and write the outcome under "Notes".

### Phase 2: storage

5. [x] Introduce a storage boundary in the app (one trait, native and web impls) for the
       recovery scenes, workspace manifest, preferences and library. All nine `std::fs` call
       sites go through it; native behaviour (atomic writes, migration) stays identical.
6. [x] Web impl on IndexedDB (or OPFS). localStorage's ~5 MB cap is too small once scenes embed
       base64 images.
7. [x] Make startup restore asynchronous on web: load, then apply through `AppHandle::update`.
       Show the empty canvas meanwhile, without a flash of wrong theme.
8. [x] Replace `on_close` flushing with `visibilitychange` / `pagehide` autosave on web.

### Phase 3: files

9. [x] Change file flows (open, save, export, image import, library import/export) from
       "path" to "file name + bytes", so both platforms share the logic.
10. [x] Web open: `<input type="file">` with the existing filters. Web save/export: Blob
        download; on Chromium, `showSaveFilePicker` so Ctrl+S can overwrite the same file.
        Prefer contributing a byte-based dialog API to rune over app-side `web-sys` code; if
        app-side, keep it in one module behind `cfg(target_arch = "wasm32")`.
11. [x] Drag-and-drop: DOM `drop` listeners (or a rune contribution) feeding the existing
        `drop_files` logic with bytes.
12. [x] Tabs: Save / Discard / Cancel on close and the "unsaved" indicator still make sense
        when there is no file path (downloads only).

### Phase 4: polish and verification

13. [x] Rebind shortcuts the browser reserves (tab new/close/switch) on web only; update the
        shortcut help and command palette.
14. [x] Size: subset Inter, run `wasm-opt -Oz`, record raw and gzip sizes. Consider loading
        export-only code (resvg, rustybuzz) lazily if it dominates.
15. [x] Page: title, favicon, a clear message when WebGPU is missing, `viewport-fit=cover`.
16. [x] Verification: a wasm `cargo check` in the standard check list, and rune's
        `scripts/web-smoke.mjs` screenshot for the web build.
17. [x] README: how to build and serve the web version, and which browsers support it.

## Open questions

- WebGPU only, or try rune's `webgl` feature? `WEB.md` says there is no WebGL fallback yet.
- Should the web build be installable (PWA, offline cache)? Out of scope unless asked.
- Fonts for CJK and emoji: web has no system fonts, so missing glyphs render as boxes.

## Notes

**Go.** The spike found one blocker outside the app: rune's `Live::new` asserts lock-free
128-bit atomics, which wasm32 lacks, so the app panicked on start (rune `dev` still does this as of
2026-11-01). `app/rusty-pencil/web/build.sh` applies a one-line patch (`rune-live-wasm.patch`)
to a cached copy of the pinned checkout for the web build only. Native builds use the pinned
git revision untouched. The fix belongs upstream; drop the patch once rune has it.

Where the plan changed:

- **Entry point (2).** No `lib.rs`: the existing binary builds for wasm32 and wasm-bindgen
  runs its `main`. A `#[wasm_bindgen(start)]` export would need macro-generated `unsafe`, which
  the workspace forbids.
- **Storage (5–7).** One `platform` module with a native and a web backend, selected by
  `cfg`, rather than a trait object. The web backend loads IndexedDB into memory before the
  app starts, so restore stays synchronous: no async restore, and no flash of the wrong theme.
- **Unload (8).** `pagehide` and `visibilitychange` run the desktop close flush through
  `AppHandle::update`, which lands within the event. IndexedDB writes call
  `transaction.commit()`; without it, a reload straight after an edit lost the edit.
- **Saving (10).** Save and export download a fresh copy each time. `showSaveFilePicker`
  (overwrite in place on Chromium) is not done; see follow-ups.
- **Shortcuts (13).** Web tabs use Alt+T, Alt+W, Alt+PageUp/PageDown and Alt+1…9. The help
  dialog lists the tab keys per platform.
- **Fonts.** `⌕` (Find) came from a system fallback font on native and rendered blank in the
  browser; it is now a drawn icon. The interface strings are checked against the web subset.
- **Size (14).** Release `.wasm`: 15.1 MB raw / 4.56 MB gzip from rune's script alone;
  9.07 MB / 4.01 MB with `wasm-opt -Oz` and the Inter subset. Lazy-loading export code was not
  pursued: wasm-bindgen has no code splitting, and the remaining size is mostly rune and wgpu.
- **Smoothness (3).** In headed Chromium (Linux, Vulkan), every pointer move during a drawing
  drag produced a frame: median 18 ms, p95 19 ms between frames, paced by the input. Wheel
  panning: median 18 ms, p95 35 ms.
- **Verification (16).** rune's `web-smoke.mjs` is specific to its `web-hello` example, so the
  app has its own `web/smoke.mjs`. Headless Chromium on Linux renders WebGPU but captures it as
  transparent; run it with `HEADED=1`.

Follow-ups, not blocking:

- [ ] Upstream the `Live` wasm fix to rune, then delete `web/rune-live-wasm.patch`.
- [ ] Upstream the web modifier fix (forward `ModifiersChanged` to the runtime) to rune, then
  delete `web/rune-web-mods.patch`.
- [ ] `showSaveFilePicker` on Chromium so Ctrl+S overwrites the same file.
- [ ] A CI workflow for the wasm check and the smoke test (the repo has no CI yet).
