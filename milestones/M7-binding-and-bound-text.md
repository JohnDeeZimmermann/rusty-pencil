# M7: Arrow binding and bound text

**Goal:** diagrams that stay connected.

**Estimate:** 2 days.

## Done when

- [ ] Arrow ends near a shape highlight it and bind on release; Ctrl disables binding.
- [ ] Moving, resizing or rotating a shape updates bound arrows.
- [ ] Arrow type: sharp, curved, elbow. Elbow arrows route orthogonally around their endpoints'
      shapes.
- [ ] Double-click a shape to type text inside it; text wraps to the container; vertical align
      top / middle / bottom.
- [ ] Double-click an arrow to add a label that follows the arrow.
- [ ] "Wrap text in container" action.

## Tasks

1. [ ] `pen-core`: `Binding { element: ElementId, focus: f32, gap: f32 }` on arrow ends; shapes keep
       a back-reference list. An invariant check (debug builds) verifies both sides agree.
2. [ ] `pen-core`: `container_id` on text; containers list bound text.
3. [ ] `pen-editor`: a "dependent update" pass that runs after any command touching a bound
       element, inside the same history entry.
4. [ ] `pen-render`: curved arrows as Catmull-Rom → cubic Bézier; elbow routing on a grid.
5. [ ] Deleting a shape unbinds its arrows; deleting a container deletes its text.

## Tests

- Move a shape 100 px: bound arrow endpoint moves, unbound end stays.
- Undo the move restores both the shape and the arrow in one step.
- Load a file with a dangling binding: it's dropped with a warning, not a crash.

## Notes
