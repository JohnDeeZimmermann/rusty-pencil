# M2: Selection, transform, history

**Goal:** select elements, move and resize them, delete them, and undo/redo every change.

**Estimate:** 1 day.

## Done when

- [x] Click selects the top-most element under the pointer; Shift+click toggles.
- [x] Drag on empty canvas draws a selection box; Ctrl+A selects all.
- [x] Drag moves the selection; arrow keys nudge 1 px, Shift+arrow 10 px.
- [x] Eight handles resize; Shift keeps aspect ratio; Alt resizes from the centre.
- [x] Del/Backspace deletes the selection.
- [x] Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y undo and redo; a whole drag is one step; selection is restored.

## Tasks

1. [x] `pen-core`: hit testing per kind. Filled shapes hit inside; unfilled shapes hit near the
       outline. Tolerance is in screen pixels divided by zoom.
2. [x] `pen-core`: `Command::Patch { id, before, after }` and `Command::Group(Vec<Command>)`.
3. [x] `pen-editor`: `Selection` (ordered set of ids) and selection-tool state machine:
       `Idle → PressOnElement | PressOnHandle | PressOnEmpty → Moving | Resizing | BoxSelecting`.
4. [x] `pen-editor`: `History` with undo/redo stacks of `(Command, SelectionBefore, SelectionAfter)`;
       a gesture opens a transaction and commits one grouped entry on pointer up. Esc cancels and
       rolls back.
5. [x] `pen-render`: overlay primitives for selection outline, handles and selection box, drawn in
       the live layer at constant screen size.
6. [x] App: route keys (`on_key` on the root, canvas focusable).

## Tests

- Hit testing: points inside, on the outline, just outside, under zoom.
- Moving and resizing through `InputEvent`s, each modifier.
- Undo after a 50-move drag reverts in one step; redo re-applies; history after a new edit drops
  the redo stack.

## Notes

- Move/resize transactions leave the document unchanged until release. Previews contain only
  touched elements; Escape/focus loss discards them and restores the starting selection.
- Static paint is split around touched elements once at drag start, preserving z-order without
  rebuilding untouched geometry during the drag. Patch versions advance on undo and redo.
- Selection feedback uses live screen rectangles. Handles remain 8 screen points and outlines
  1 screen point at every zoom; camera events stay live-only. Feedback has no enter/exit fade.
- Unfilled shapes select near their visible outline. Once selected, their interior is a move
  target. Marquee selects elements whose bounds overlap the box on release, including partially
  enclosed paths; Shift adds. Membership is scanned once on release so marquee animation does
  not walk the document each frame.
- Arrow-key nudges are 1 or 10 screen points, converted through the camera. Shift constrains
  moves to one axis. Eight-handle resize applies to one element; multi-resize remains M6.
- Reproduce visuals with `scripts/selection.json` in the app: snapshot times 1.85 (selected),
  2.6 (moving), 3.5 (resizing), 4.8 (undo), 5.5 (marquee), 6.8 (undo deletion).
- `scripts/marquee-overlap.json`: snapshot at 2.8 (box crossing a freehand stroke and enclosing
  a rectangle) and 3.3 (both selected, with a separate rectangle left unselected).
