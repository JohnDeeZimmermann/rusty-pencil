# M9: Frames, library, polish

**Goal:** the remaining Excalidraw features.

**Estimate:** 2–3 days. Split into smaller milestones if it grows.

## Done when

- [ ] Frame tool (`F`): named frames clip and own their children; drag elements in/out; export a
      single frame.
- [ ] Personal library: add selection, drag items onto the canvas, import/export library files.
- [ ] Laser pointer (`K`) and lasso selection.
- [ ] Snap to objects (Alt+S) with guide lines.
- [ ] Zen mode (Alt+Z), view mode (Alt+R), stats panel (Alt+/) with editable x/y/w/h/angle.
- [ ] Find text (Ctrl+F), command palette (Ctrl+/), shortcut help (`?`), eye-dropper (`I`).
- [ ] Hyperlinks on elements (Ctrl+K, opens in the system browser) and links to other elements.

## Tasks

1. [ ] `pen-core`: `Kind::Frame { name }` and `frame_id` on elements; render clipping in
       `pen-render`.
2. [ ] Library stored in the platform data dir, same file format as scenes.
3. [ ] Command palette lists the M6 `Action` registry.
4. [ ] Snapping: spatial index (evaluate `rstar`) shared with hit testing once scenes grow past
       ~1 000 elements.

## Multi-tab follow-up (user-requested priority)

- [x] Add a tab strip with new/close controls, overflow scrolling and keyboard navigation.
- [x] Keep document, history, selection, camera and drawing defaults independent per tab.
- [x] Open files in new tabs and focus files already open; bind save dialogs to their tab.
- [x] Show unsaved changes and offer Save, Discard and Cancel when closing.
- [x] Recover all open canvases, file paths and the active tab across restarts.
- [x] Exercise real input, dialog callbacks and recovery through headless app tests.
- [x] Review light/dark screenshots and the unsaved-tab close prompt.

## Notes

Multi-tab work was explicitly prioritized before the remaining M9 tools. Inactive tabs own
their editor and retained scene; switching moves that state without cloning documents.
Recovery uses per-tab scene files plus an atomic JSON manifest. Existing single-canvas
recovery migrates on first use. `serde` and `serde_json`, already in the workspace dependency
graph, serialize the manifest instead of maintaining a custom metadata parser.
