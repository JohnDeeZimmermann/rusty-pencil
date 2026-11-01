# M8: Images, export, import

**Goal:** get pictures in and drawings out.

**Estimate:** 1–2 days.

## Done when

- [x] Image tool (`9`), paste and drag-and-drop insert images; images can be cropped.
- [x] Images are stored inside the scene file (deduplicated by content hash).
- [x] Export PNG: 1×/2×/3×, background on/off, dark mode, selection only.
- [x] Export SVG.
- [ ] Optional: embed the scene in the PNG/SVG and re-open it.
- [x] Copy selection as PNG / SVG to the clipboard.
- [x] Open `.excalidraw` files (elements we support; others skipped with a notice).

## Tasks

1. [x] `pen-core`: `Kind::Image { file_id, crop }` and `files: BTreeMap<FileId, BinaryFile>`.
2. [x] `pen-render`: SVG backend from `DrawList` (no rune needed).
3. [x] PNG: render headless (rune `--snap` path / offscreen) or a CPU rasteriser (evaluate
       `tiny-skia`) from `DrawList`. Decide and note why.
4. [x] `pen-core::import::excalidraw`: map their JSON schema to ours; unit-test against sample files
       in `tests/fixtures/`.
5. [x] Bound every image cache by entries and decoded bytes.

## Tests

- SVG export of a fixture scene matches a golden file.
- Excalidraw fixture with every element type imports without error.

## Notes

- PNG uses `resvg` and its `tiny-skia` CPU backend over the same SVG paint as SVG export.
  This needs no window or GPU and shares geometry, cropping and bundled fonts across formats.
  `rustybuzz` measures export line breaks; SVG text is outlined for portable appearance.
- Images accept PNG/JPEG, at most 16 MiB encoded and 16 megapixels each. Scene v2 stores
  base64 bytes once per BLAKE3 hash; v0/v1 scenes still load with an empty file map.
  Saved scenes omit unused assets. `imagesize` reads dimensions before any decoding.
- The app's decoded cache is capped at 64 sources and 128 MiB; excess sources in imported
  scenes show a placeholder. PNG output is capped at 64 megapixels. Pan/zoom retains pixels.
- Select an image to trim 10% from any edge or reset its non-destructive crop. Trimming
  preserves source scale and rotation and is one undo step, as are image insertions/pastes.
- File → Export (Ctrl+Shift+E) offers scale, background, dark mode and selection-only
  PNG/SVG. Copy PNG uses Ctrl+Shift+C; Copy SVG uses Ctrl+Alt+Shift+C. Bound labels are
  included with selected containers. SVG is copied as source text with a rich HTML form;
  Rune has no arbitrary `image/svg+xml` clipboard slot.
- Import follows the upstream [element schema](https://github.com/excalidraw/excalidraw/blob/master/packages/element/src/types.ts).
  Shapes, paths, text and images retain supported styles, groups, seeds, pressure and
  supported container/arrow relationships. Unsupported element types are counted in a notice.
- Optional scene embedding in exported PNG/SVG is deferred; editable scenes use the native
  scene format. All mandatory M8 acceptance items are implemented.
- Verification: SVG golden, all-types import fixture, PNG pixel/scale/alpha/crop checks,
  editor history tests, bounded-cache test and Rune input/dialog/clipboard tests.
  `scripts/m8-images.json` demonstrates image drop, text, crop and the export panel.
