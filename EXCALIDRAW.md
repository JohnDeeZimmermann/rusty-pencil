# Excalidraw feature checklist (offline core)

Everything Excalidraw does locally. Left out on purpose: live collaboration, shareable links,
Excalidraw+ cloud, the public library browser, web embeds and AI features ("magic frame",
text-to-diagram).

Tags say which milestone ships the item: `M1` … `M9`. Tick the box when it works in the app.

## Tools

- [x] Selection tool (`V` / `1`) — M2
- [x] Rectangle (`R` / `2`) — M1
- [x] Diamond (`D` / `3`) — M1
- [x] Ellipse (`O` / `4`) — M1
- [x] Arrow (`A` / `5`) — M3
- [x] Line (`L` / `6`) — M3
- [x] Freedraw / pen (`P` / `7`) — M3
- [x] Text (`T` / `8`) — M4
- [ ] Image (`9`) — M8
- [x] Eraser (`E` / `0`), hold Alt to un-erase — M6
- [x] Hand / pan tool (`H`) — M3
- [ ] Frame tool (`F`) — M9
- [ ] Laser pointer (`K`) — M9
- [ ] Lasso selection — M9
- [x] Tool lock (`Q`): stay on the tool after drawing — M5
- [x] Shift while drawing: square / circle / 45° snapping — M1
- [x] Alt while drawing: draw from the centre — M1

## Element properties

- [x] Stroke colour (palette + custom hex) — M5
- [x] Background colour (palette + custom hex, transparent) — M5
- [x] Fill style: hachure, cross-hatch, solid — M5
- [x] Stroke width: thin, bold, extra bold — M5
- [x] Stroke style: solid, dashed, dotted — M5
- [x] Sloppiness: architect, artist, cartoonist — M4
- [x] Edges: sharp, round — M5
- [x] Opacity slider — M5
- [x] Font family: hand-drawn, normal, code — M4
- [x] Font size: S, M, L, XL (+ Ctrl+Shift+`<` / `>`) — M4
- [x] Text align: left, centre, right — M4
- [x] Vertical align for text inside a shape — M7
- [x] Arrowheads at start/end: none, arrow, bar, dot, triangle, circle, diamond (filled + outline) — M5
- [x] Arrow type: sharp, curved, elbow — M7
- [x] Defaults: new elements use the last chosen properties — M5
- [x] Copy styles / paste styles (Ctrl+Alt+C / V) — M6

## Selecting and editing

- [x] Click to select, Shift+click to add/remove — M2
- [x] Drag a box to select overlapping elements, including partially enclosed paths — M2
- [x] Select all (Ctrl+A) — M2
- [x] Move by dragging; arrow keys nudge 1 px, Shift+arrow 10 px — M2
- [x] Resize with 8 handles; Shift keeps aspect ratio; Alt resizes from centre — M2
- [x] Rotate with the handle; Shift snaps to 15° — M6
- [x] Multi-element resize and rotate — M6
- [x] Flip horizontal / vertical (Shift+H / Shift+V) — M6
- [x] Delete (Del / Backspace) — M2
- [x] Duplicate (Ctrl+D) and Alt+drag duplicate — M6
- [x] Copy, cut, paste within the app — M6
- [x] Paste plain text as a text element — M6
- [x] Group / ungroup (Ctrl+G / Ctrl+Shift+G), nested groups — M6
- [x] Double-click into a group to edit one member — M6
- [x] Lock / unlock elements — M6
- [x] Edit line/arrow points (Ctrl+Enter or Ctrl+double-click), add and delete points — M3
- [x] Multi-point lines and arrows by clicking point to point — M3
- [ ] Hyperlink on an element (Ctrl+K), opens in the system browser — M9
- [ ] Link to another element in the scene — M9

## Text

- [x] Click to place, type, Esc or click away to finish — M4
- [x] Multi-line text, auto-sizing box — M4
- [x] Double-click text to edit it; double-click empty canvas to write — M4
- [x] IME / non-Latin input — M4
- [x] Text inside shapes (double-click a shape), wraps to the container — M7
- [x] Labels on arrows (and lines) — M7
- [x] "Wrap text in container" action — M7

## Arrows and binding

- [x] Rebind placed arrows by dragging selected endpoint handles — M7

- [x] Arrow ends snap to and bind to shapes — M7
- [x] Bound arrows follow when shapes move or resize — M7
- [x] Hold Ctrl to draw without binding — M7
- [x] Elbow arrows route around shapes — M7

## Arranging

- [x] Send backward / to back, bring forward / to front — M6
- [x] Align left, centre, right, top, middle, bottom — M6
- [x] Distribute horizontally / vertically — M6
- [ ] Snap to objects (Alt+S) with guide lines — M9
- [x] Grid toggle (Ctrl+') and snap to grid — M5

## Canvas and view

- [x] Infinite canvas — M0
- [x] Pan: wheel, Space+drag, middle-mouse drag, hand tool — M3
- [x] Zoom: Ctrl+wheel, pinch, Ctrl+`+` / `-`, zoom indicator — M3
- [x] Reset zoom (Ctrl+0), zoom to fit (Shift+1), zoom to selection (Shift+2) — M3
- [x] "Scroll back to content" button — M3
- [x] Dark mode (Alt+Shift+D) — M5
- [x] Canvas background colour — M5
- [ ] Zen mode (Alt+Z): hide panels — M9
- [ ] View mode (Alt+R): read only — M9
- [ ] Stats panel (Alt+/): edit x, y, w, h, angle — M9
- [ ] Find text on canvas (Ctrl+F) — M9
- [ ] Command palette (Ctrl+/) — M9
- [x] Context menu (right-click) — M6
- [ ] Keyboard shortcut help (`?`) — M9
- [ ] Eye-dropper for colours (`I`) — M9

## Frames

- [ ] Create and name frames — M9
- [ ] Frames clip their children; moving a frame moves its children — M9
- [ ] Add / remove elements from a frame by dragging — M9
- [ ] Export a single frame — M9

## Images

- [ ] Insert from file — M8
- [ ] Paste and drag-and-drop images — M8
- [ ] Crop an image — M8
- [ ] Images stored inside the scene file — M8

## History

- [x] Undo / redo (Ctrl+Z / Ctrl+Shift+Z, Ctrl+Y) — M2
- [x] A whole drag or edit is one undo step — M2
- [x] Undo restores the selection too — M2

## Files and export

- [x] Top-left File and Preferences menus, without overlapping the shape inspector, for file actions and canvas settings — M5 follow-up

- [x] Save / Save as scene file (JSON) — M4
- [x] Open scene file, also by drag-and-drop onto the window — M4
- [x] Autosave and restore the last scene on launch — M4
- [x] Reset canvas — M4
- [ ] Import `.excalidraw` files — M8
- [ ] Export PNG: scale 1×/2×/3×, background on/off, dark mode, selection only — M8
- [ ] Export SVG — M8
- [ ] Embed the scene in exported PNG/SVG, re-open it later — M8
- [ ] Copy selection as PNG / SVG to the clipboard — M8

## Library

- [ ] Add selection to personal library — M9
- [ ] Drag library items onto the canvas — M9
- [ ] Import / export library files — M9
