# rusty-pencil in the browser

The same app that runs on the desktop also builds as a static WebAssembly page. It draws through
WebGPU, keeps your drawings in the browser's own storage, and never talks to a server: the
page is just files, so any static host (or `localhost`) serves it.

## Build and serve

You need the `wasm32-unknown-unknown` target and the `wasm-bindgen` CLI at the version pinned in
`Cargo.lock`. The build script tells you the exact install command if it is missing. Running
`wasm-opt` from [Binaryen](https://github.com/WebAssembly/binaryen) is optional but shrinks the
download by about a third.

```sh
rustup target add wasm32-unknown-unknown
app/rusty-pencil/web/build.sh            # release build into target/web
python3 -m http.server 8000 -d target/web
```

Then open `http://localhost:8000`. WebGPU only works on secure origins, so any real deployment
needs HTTPS. The script copies the page shell and favicon from
`/home/johndee/.t3/projects/drusty-pen/app/rusty-pencil/web/` next to the generated `app.js` and
`app_bg.wasm`.

The release `.wasm` is about **9.1 MB raw and 4.0 MB gzipped** after `wasm-opt -Oz`. The
browser build bundles a subset of Inter (see `assets/fonts/README.md`) and leaves out rune's
own fallback fonts.

### The temporary rune patch

At the pinned rune revision, every `rune::Live` value asserts lock-free 128-bit atomics, which
wasm32 does not have, so an unpatched build panics on start. The build script copies the pinned
rune checkout to `~/.cache/rusty-pencil/web-rune/`, applies `rune-live-wasm.patch` (one line:
skip the assert on wasm32, where rune runs on a single thread), and points only the web build
at that copy. Native builds always use the pinned git revision untouched. Delete the patch and
that part of `build.sh` once rune fixes this upstream.

## Supported browsers

Anything with WebGPU: current Chrome and Edge, Safari 26, and Firefox 141 or later on Windows.
There is no WebGL fallback. Without WebGPU the page shows a message instead of the canvas.

## What differs from the desktop

Most of the app behaves identically, because both builds share one code path. The differences
all live in `app/rusty-pencil/src/platform/`:

| | Desktop | Browser |
|---|---|---|
| Autosave, tabs, preferences, library | Files in the user data folder | IndexedDB, database `rusty-pencil` |
| Open, import | Native file dialog | The browser's file picker |
| Save, export | Save dialog, overwrites the file | A download named after the canvas |
| New / close / next tab | Ctrl+T, Ctrl+W, Ctrl+Tab | Alt+T, Alt+W, Alt+PageDown |
| Switch to tab 1–9 | Ctrl+1 … Ctrl+9 | Alt+1 … Alt+9 |
| Missing glyphs | System fonts fill in | Shown as boxes |

Saving in the browser always downloads a new copy, since a page cannot overwrite a file on
disk. The tab keeps the downloaded name, and the next Ctrl+S downloads under it again.

File → Open recent keeps the ten most recently opened or saved scenes in browser storage.
Reopening uses that local copy, including after its tab closes or the page reloads. Entries with
the same filename share the most recent copy. Clear history removes the list and those copies;
open tabs and downloaded files remain available.

Pending edits are flushed when the page is hidden or closed (`pagehide` and
`visibilitychange`), and each IndexedDB write commits at once. A reload right after an edit
keeps it. Everything stays in this browser profile: clearing site data clears your canvases,
so save anything you want to keep.

## Checking a web build

`app/rusty-pencil/web/smoke.mjs` drives the built page through Playwright with real input. It
draws, reloads, toggles dark mode, saves, opens a scene and an `.excalidraw` file, drops an
image, copies and pastes (scene, plain text and image), exports PNG and SVG, and round-trips the
library. It also reopens a recent scene after a reload and tab close, and checks clearing history.
It writes screenshots and frame timings to `target/web-proof`.

```sh
HEADED=1 node app/rusty-pencil/web/smoke.mjs target/web target/web-proof
```

Headless Chromium on Linux renders WebGPU but captures the canvas as transparent, so
`HEADED=1` opens a real window for screenshots. Set `PLAYWRIGHT_MODULE` and `CHROMIUM_PATH` when
Playwright or its browser lives somewhere else.
