# M4: Text, hand-drawn style, files

**Goal:** write text, see the hand-drawn look, and keep work between launches.

**Estimate:** 1–2 days.

## Done when

- [x] Text tool (`T`): click to place, type multi-line, Esc or click away finishes; empty text is
      removed. Double-click text edits it. IME works.
- [x] Font family (hand-drawn, normal, code), size S/M/L/XL, align left/centre/right;
      Ctrl+Shift+`<`/`>` changes size.
- [x] Sloppiness architect / artist / cartoonist renders a rough, stable, hand-drawn outline.
- [x] Ctrl+S / Ctrl+Shift+S save, Ctrl+O opens, dropping a file on the window opens it.
- [x] The last scene restores on launch (autosave); "Reset canvas" clears it with confirmation.

## Tasks

1. [x] `pen-core`: `Kind::Text { content, font, size, align }`; measured size stored on the
       element and refreshed by the app after layout.
2. [x] App: text editing via rune `text_input` / `TextEdit` placed at the element's screen rect;
       commit as one history entry.
3. [x] Bundle fonts under `assets/fonts/` (check licences: OFL or similar).
4. [x] `pen-render`: rough generator seeded by `element.seed` (evaluate the `roughr` crate vs own
       port); cache output by `(id, version)`, sampled for all zooms (see Notes).
5. [x] `pen-core::file`: `SceneFile { format: "drusty-pen", version: u32, elements, app_state }` with
       serde; `load` migrates old versions and returns typed errors.
6. [x] App: file dialogs (`rune::FileDialogs`), `on_drop_files`, autosave to the platform data dir
       debounced to 1 s after the last change.

## Tests

- File round trip: save → load gives an equal document, for every element kind.
- Loading garbage, a newer version and a truncated file each returns a distinct `Err`.
- Rough output is identical for the same seed and differs for another seed.

## Notes

- Rune's `TextEdit` owns caret movement, selection, clipboard input and IME composition.
  A transparent intrinsic label sizes the editing overlay even before preedit commits.
  The app measures committed text with the same bundled fonts and stores world bounds.
  Finishing, deleting empty text, and changing text properties each use one history entry.
  Text resizes uniformly and clamps font size to the file format's valid range.
- Bundled Caveat, Inter and Roboto Mono are under SIL OFL 1.1; each licence ships alongside
  its font in `assets/fonts/`. Rune retains system fallback for other writing systems.
- Evaluated [roughr](https://docs.rs/roughr/0.14.0/roughr/), whose generator exposes its own
  curve and fill operations. M4 uses a small original contour perturbation generator instead:
  it reuses our existing rounded outlines and path paint without another geometry adapter.
  Architect is clean, Artist restrained, Cartoonist more expressive. Noise depends only on
  the element seed, with fixed frequencies so resizing stays continuous. Drafts use the
  eventual element seed, avoiding a change in the outline on release.
- Geometry stays camera independent, sampled finely for the maximum zoom. The existing
  `(id, version)` cache therefore needs no zoom bucket and no scene rebuild on pan or zoom.
  Text primitives preserve document z-order by splitting backend paint around text runs.
- Scene version 1 includes elements, camera, stroke defaults and text defaults. Version 0
  migrates missing defaults. The loader validates identities, numeric geometry, paths, font
  sizes and camera before replacing the current document; a future version is reported
  before decoding its element kinds. Opening starts a fresh undo history and renderer cache.
- `serde`/`serde_json` replace custom JSON parsing and serialization; `directories` supplies
  platform data locations; `tempfile` supplies atomic replacement and cleanup on failure.
  Rune's `native-file-dialog` feature enables platform dialogs (XDG portal on Linux).
- Autosave writes `autosave.drusty-pen` to the platform local data directory one second after
  the last change, including active text drafts without committing them. Close flushes pending
  work. A failed write leaves it pending for another edit or close, without spinning.
  Explicit Save and Save as write named files; autosave remains separate recovery storage.
- Reset uses a modal confirmation, clears recovery, resets camera and defaults, and detaches
  the named save file. Scripted/headless runs leave normal recovery untouched; `--data-dir`
  opts into a separate directory for restore tests. `--open` opens a scene directly.
- Visual fixture: `app/drusty-pen/scripts/m4-text-style.json`, snapshot at 18.6 seconds.
  Workspace tests cover file round trips for all kinds, corruption and migration, deterministic
  outlines, unchanged draft/commit paint, native text/IME dispatch, font controls, live camera,
  save/open/drop, reset cancellation, debounce, active-draft recovery and write failures.

- The pinned Rune renderer limits inherited glyph raster scale to 1/16 through 4. Document
  text uses an equivalent 8x layout with an inverse transform and the original optical size;
  3000% zoom then rasterizes at 3.75x instead of magnifying a small bitmap. World bounds and
  the live-only camera contract stay unchanged. A runtime test asserts 1:1 glyph sampling at
  maximum zoom after settling, with no view commits.
- Final release fixture (`--scale 1`, 1,117 scripted frames): frame CPU mean **0.246 ms**, p95
  **0.643 ms** on this machine. This is a small scene, not a large-document benchmark.
  All **149 workspace tests** pass. Screenshots checked the finished scene, active text
  editing, reset confirmation, launch recovery and sharp text at **3000%**.

- Inspector refinement: the left panel now appears only for the text tool, selected text or
  an active text edit. Font family and alignment use outline icons with descriptive tooltips;
  size choices share one row. Sloppiness cycles from a stroke icon in the toolbar, retaining
  the M4 styling feature without a permanent sidebar.
