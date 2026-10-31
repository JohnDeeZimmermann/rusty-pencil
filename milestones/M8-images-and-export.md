# M8: Images, export, import

**Goal:** get pictures in and drawings out.

**Estimate:** 1–2 days.

## Done when

- [ ] Image tool (`9`), paste and drag-and-drop insert images; images can be cropped.
- [ ] Images are stored inside the scene file (deduplicated by content hash).
- [ ] Export PNG: 1×/2×/3×, background on/off, dark mode, selection only.
- [ ] Export SVG.
- [ ] Optional: embed the scene in the PNG/SVG and re-open it.
- [ ] Copy selection as PNG / SVG to the clipboard.
- [ ] Open `.excalidraw` files (elements we support; others skipped with a notice).

## Tasks

1. [ ] `pen-core`: `Kind::Image { file_id, crop }` and `files: BTreeMap<FileId, BinaryFile>`.
2. [ ] `pen-render`: SVG backend from `DrawList` (no rune needed).
3. [ ] PNG: render headless (rune `--snap` path / offscreen) or a CPU rasteriser (evaluate
       `tiny-skia`) from `DrawList`. Decide and note why.
4. [ ] `pen-core::import::excalidraw`: map their JSON schema to ours; unit-test against sample files
       in `tests/fixtures/`.
5. [ ] Bound every image cache by entries and decoded bytes.

## Tests

- SVG export of a fixture scene matches a golden file.
- Excalidraw fixture with every element type imports without error.

## Notes
