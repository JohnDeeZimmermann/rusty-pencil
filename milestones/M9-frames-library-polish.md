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

## Notes
