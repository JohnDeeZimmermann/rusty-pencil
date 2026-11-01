# M10: Run in the browser

**Goal:** the same app runs as a static WebAssembly page, with local persistence and file
open/save, without forking the code base. Native stays the primary target; nothing on native
may regress.

**Estimate:** 3–4 days. Phase 1 is a half-day spike; decide after it whether to continue.

## Done when

- [ ] `scripts/web-build.sh` (rune's) produces a static folder that serves from `localhost` and
      shows the canvas in a WebGPU browser, with the same visuals as native.
- [ ] Drawing, selection, pan/zoom, text, properties, tabs and undo work in the browser with
      native-equivalent smoothness.
- [ ] Autosave, workspace recovery, preferences and the personal library survive a page reload.
- [ ] Open a scene, `.excalidraw` or library file from disk; save scenes and export PNG/SVG and
      library files as downloads.
- [ ] Drag-and-drop of images and scene files onto the canvas works.
- [ ] Copy and paste (rich scene, plain text, images) work through the browser clipboard.
- [ ] No shortcut depends on a key the browser reserves (Ctrl+W, Ctrl+T, Ctrl+N).
- [ ] CI-style check: `cargo check -p rusty-pencil --target wasm32-unknown-unknown` passes, and
      rune's Playwright smoke test captures a screenshot of the web build.

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

1. [ ] Manifest: move `native-file-dialog`, `directories` and `tempfile` under
       `cfg(not(target_arch = "wasm32"))`. On wasm, drop rune's `embedded-fonts` (the app
       supplies its own through `AppSpec::fonts`).
2. [ ] Add `app/rusty-pencil/src/lib.rs` with `crate-type = ["cdylib", "rlib"]`; `main.rs`
       and a `#[wasm_bindgen(start)]` function both call one shared `run()`.
3. [ ] Build with rune's `scripts/web-build.sh rusty-pencil` and check the page in Chromium.
       Record frame smoothness for draw, drag and pan/zoom against native.
4. [ ] Decide go/no-go and write the outcome under "Notes".

### Phase 2: storage

5. [ ] Introduce a storage boundary in the app (one trait, native and web impls) for the
       recovery scenes, workspace manifest, preferences and library. All nine `std::fs` call
       sites go through it; native behaviour (atomic writes, migration) stays identical.
6. [ ] Web impl on IndexedDB (or OPFS). localStorage's ~5 MB cap is too small once scenes embed
       base64 images.
7. [ ] Make startup restore asynchronous on web: load, then apply through `AppHandle::update`.
       Show the empty canvas meanwhile, without a flash of wrong theme.
8. [ ] Replace `on_close` flushing with `visibilitychange` / `pagehide` autosave on web.

### Phase 3: files

9. [ ] Change file flows (open, save, export, image import, library import/export) from
       "path" to "file name + bytes", so both platforms share the logic.
10. [ ] Web open: `<input type="file">` with the existing filters. Web save/export: Blob
        download; on Chromium, `showSaveFilePicker` so Ctrl+S can overwrite the same file.
        Prefer contributing a byte-based dialog API to rune over app-side `web-sys` code; if
        app-side, keep it in one module behind `cfg(target_arch = "wasm32")`.
11. [ ] Drag-and-drop: DOM `drop` listeners (or a rune contribution) feeding the existing
        `drop_files` logic with bytes.
12. [ ] Tabs: Save / Discard / Cancel on close and the "unsaved" indicator still make sense
        when there is no file path (downloads only).

### Phase 4: polish and verification

13. [ ] Rebind shortcuts the browser reserves (tab new/close/switch) on web only; update the
        shortcut help and command palette.
14. [ ] Size: subset Inter, run `wasm-opt -Oz`, record raw and gzip sizes. Consider loading
        export-only code (resvg, rustybuzz) lazily if it dominates.
15. [ ] Page: title, favicon, a clear message when WebGPU is missing, `viewport-fit=cover`.
16. [ ] Verification: a wasm `cargo check` in the standard check list, and rune's
        `scripts/web-smoke.mjs` screenshot for the web build.
17. [ ] README: how to build and serve the web version, and which browsers support it.

## Open questions

- WebGPU only, or try rune's `webgl` feature? `WEB.md` says there is no WebGL fallback yet.
- Should the web build be installable (PWA, offline cache)? Out of scope unless asked.
- Fonts for CJK and emoji: web has no system fonts, so missing glyphs render as boxes.

## Notes
