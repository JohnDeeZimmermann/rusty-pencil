# M4: Text, hand-drawn style, files

**Goal:** write text, see the hand-drawn look, and keep work between launches.

**Estimate:** 1–2 days.

## Done when

- [ ] Text tool (`T`): click to place, type multi-line, Esc or click away finishes; empty text is
      removed. Double-click text edits it. IME works.
- [ ] Font family (hand-drawn, normal, code), size S/M/L/XL, align left/centre/right;
      Ctrl+Shift+`<`/`>` changes size.
- [ ] Sloppiness architect / artist / cartoonist renders a rough, stable, hand-drawn outline.
- [ ] Ctrl+S / Ctrl+Shift+S save, Ctrl+O opens, dropping a file on the window opens it.
- [ ] The last scene restores on launch (autosave); "Reset canvas" clears it with confirmation.

## Tasks

1. [ ] `pen-core`: `Kind::Text { content, font, size, align }`; measured size stored on the
       element and refreshed by the app after layout.
2. [ ] App: text editing via rune `text_input` / `TextEdit` placed at the element's screen rect;
       commit as one history entry.
3. [ ] Bundle fonts under `assets/fonts/` (check licences: OFL or similar).
4. [ ] `pen-render`: rough generator seeded by `element.seed` (evaluate the `roughr` crate vs own
       port); cache output by `(id, version, zoom bucket)`.
5. [ ] `pen-core::file`: `SceneFile { format: "drusty-pen", version: u32, elements, app_state }` with
       serde; `load` migrates old versions and returns typed errors.
6. [ ] App: file dialogs (`rune::FileDialogs`), `on_drop_files`, autosave to the platform data dir
       debounced to 1 s after the last change.

## Tests

- File round trip: save → load gives an equal document, for every element kind.
- Loading garbage, a newer version and a truncated file each returns a distinct `Err`.
- Rough output is identical for the same seed and differs for another seed.

## Notes
