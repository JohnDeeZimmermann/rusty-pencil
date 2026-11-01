# M9: Frames, library, polish

**Goal:** the remaining Excalidraw features.

**Estimate:** 2–3 days. Split into smaller milestones if it grows.

## Done when

- [x] Frame tool (`F`): named frames clip and own their children; drag elements in/out; export a
      single frame.
- [x] Personal library: add selection, drag items onto the canvas, import/export library files.
- [x] Laser pointer (`K`) and lasso selection.
- [x] Snap to objects (Alt+S) with guide lines.
- [x] Zen mode (Alt+Z), view mode (Alt+R), stats panel (Alt+/) with editable x/y/w/h/angle.
- [x] Find text (Ctrl+F), command palette (Ctrl+/), shortcut help (`?`), eye-dropper (`I`).
- [x] Hyperlinks on elements (Ctrl+K, opens in the system browser) and links to other elements.

## Tasks

1. [x] `pen-core`: `Kind::Frame { name }` and `frame_id` on elements; render clipping in
       `pen-render`.
2. [x] Library stored in the platform data dir, same file format as scenes.
3. [x] Command palette lists the M6 `Action` registry.
4. [x] Snapping: spatial index (evaluate `rstar`) shared with hit testing once scenes grow past
       ~1 000 elements.

## Multi-tab follow-up (user-requested priority)

- [x] Add a tab strip with new/close controls, overflow scrolling and keyboard navigation.
- [x] Keep document, history, selection, camera and drawing defaults independent per tab.
- [x] Open files in new tabs and focus files already open; bind save dialogs to their tab.
- [x] Show unsaved changes and offer Save, Discard and Cancel when closing.
- [x] Recover all open canvases, file paths and the active tab across restarts.
- [x] Exercise real input, dialog callbacks and recovery through headless app tests.
- [x] Review light/dark screenshots and the unsaved-tab close prompt.
- [x] Persist light/dark appearance globally, without storing it in or dirtying canvases.

## Notes

Multi-tab work was explicitly prioritized before the remaining M9 tools. Inactive tabs own
their editor and retained scene; switching moves that state without cloning documents.
Recovery uses per-tab scene files plus an atomic JSON manifest. Existing single-canvas
recovery migrates on first use. `serde` and `serde_json`, already in the workspace dependency
graph, serialize the manifest instead of maintaining a custom metadata parser.

## Implementation and verification

Frames are flat, named, axis-aligned containers. An element enters or leaves a frame when its
centre crosses the boundary, including during the live preview. Moving a frame moves its
children; resizing changes the clipping window. Frame deletion, duplication, clipboard and
library placement preserve ownership in one undo step. Frame names and local/web links
round-trip in scene version 3; older scenes receive empty ownership and link fields.
Excalidraw imports now retain supported frames, names, children and web links.

Selected frames export at their exact bounds, without frame labels or selection chrome. The
same nested clipping primitives drive the Rune canvas and CPU PNG/SVG exports. Pixel tests
check clipping, transparency, empty frames and exact export dimensions.

The personal library stores validated scene snippets, including embedded images, in
`library.drusty-library` beside workspace recovery in the platform data directory. Library
files wrap those same scene files in a versioned collection. Import validates the entire
collection before changing the library; writes are atomic. Thumbnails are memoized, and
drops remap element, frame, group, arrow, text and local-link identities. Screenshot runs
use memory unless an isolated `--data-dir` is supplied.

Lasso (`S`) selects visible element centres and supports Shift to add. Laser (`K`) trails
fade over 0.9 seconds and never enter history or dirty the document. Eye-dropper (`I`) samples
stored sRGB colours, including cropped/rotated image pixels; Shift applies the fill colour.
Zen, view, stats and object snapping are available through Preferences and their shortcuts.
View mode rejects document mutations at the editor boundary while preserving navigation,
search and link following. Stats edits use world-point geometry and degrees, applied on Enter.

Ctrl+F searches text and frame names; Ctrl+/ searches the complete M6 Action registry, tools
and view modes. Arrow keys navigate results and Enter runs them. `?` opens shortcut help.
Ctrl+K accepts an HTTP(S) URL, `#element-id`, or an empty value to remove a link. Use the
selection's Open link control or Ctrl+click an element to follow its destination. Element ids
are shown in Stats. Web links use Rune's system-browser transport.

For object snapping, a new `rstar` dependency was weighed against extending the existing
deterministic bounds hierarchy. Kept the dependency-free hierarchy and added sorted edge/centre
anchors shared with hit testing. Drag targets are filtered and duplicate anchors merged once
on press; each pointer move checks nearest anchors using binary searches. Release measurements
for 4,000 moves: 1,000 elements averaged 0.314 µs (p95 0.370 µs); 10,000 averaged 0.363 µs
(p95 0.411 µs), excluding rendering. Pan and zoom keep using retained live transforms, including
laser/lasso feedback.

InputEvent tests cover tools, ownership, undo, preview cancellation, clipboard, links, lasso,
laser fading, snapping, migration and read-only guards. Rune tests exercise fields, search focus,
the Action palette, links, image sampling, library drag/drop, dialogs and library recovery.
`app/rusty-pencil/scripts/m9-frames.json` reproduces light frames (6.0 s), Stats (6.5 s), the
library (7.6 s), dark library (8.4 s), palette (9.2 s), help (10.4 s), laser (11.6 s) and zen
mode (13.2 s).
