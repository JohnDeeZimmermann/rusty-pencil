# M2: Selection, transform, history

**Goal:** select elements, move and resize them, delete them, and undo/redo every change.

**Estimate:** 1 day.

## Done when

- [ ] Click selects the top-most element under the pointer; Shift+click toggles.
- [ ] Drag on empty canvas draws a selection box; Ctrl+A selects all.
- [ ] Drag moves the selection; arrow keys nudge 1 px, Shift+arrow 10 px.
- [ ] Eight handles resize; Shift keeps aspect ratio; Alt resizes from the centre.
- [ ] Del/Backspace deletes the selection.
- [ ] Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y undo and redo; a whole drag is one step; selection is restored.

## Tasks

1. [ ] `pen-core`: hit testing per kind. Filled shapes hit inside; unfilled shapes hit near the
       outline. Tolerance is in screen pixels divided by zoom.
2. [ ] `pen-core`: `Command::Patch { id, before, after }` and `Command::Group(Vec<Command>)`.
3. [ ] `pen-editor`: `Selection` (ordered set of ids) and selection-tool state machine:
       `Idle → PressOnElement | PressOnHandle | PressOnEmpty → Moving | Resizing | BoxSelecting`.
4. [ ] `pen-editor`: `History` with undo/redo stacks of `(Command, SelectionBefore, SelectionAfter)`;
       a gesture opens a transaction and commits one grouped entry on pointer up. Esc cancels and
       rolls back.
5. [ ] `pen-render`: overlay primitives for selection outline, handles and selection box, drawn in
       the live layer at constant screen size.
6. [ ] App: route keys (`on_key` on the root, canvas focusable).

## Tests

- Hit testing: points inside, on the outline, just outside, under zoom.
- Moving and resizing through `InputEvent`s, each modifier.
- Undo after a 50-move drag reverts in one step; redo re-applies; history after a new edit drops
  the redo stack.

## Notes
