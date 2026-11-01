# M7: Arrow binding and bound text

**Goal:** diagrams that stay connected.

**Estimate:** 2 days.

## Done when

- [ ] Arrow ends near a shape highlight it and bind on release; Ctrl disables binding.
- [ ] Moving, resizing or rotating a shape updates bound arrows.
- [ ] Arrow type: sharp, curved, elbow. Elbow arrows route orthogonally around their endpoints'
      shapes.
- [ ] Double-click a shape to type text inside it; text wraps to the container; vertical align
      top / middle / bottom. *(Shipped early, ahead of M5/M6, at the user's request. Vertical
      align is still to do; text is always centred.)*
- [x] Double-click an arrow to add a label that follows the arrow.
- [ ] "Wrap text in container" action.

## Tasks

1. [ ] `pen-core`: `Binding { element: ElementId, focus: f32, gap: f32 }` on arrow ends; shapes keep
       a back-reference list. An invariant check (debug builds) verifies both sides agree.
2. [x] `pen-core`: `container_id` on text; containers list bound text. *(See notes: only the
       text stores the link.)*
3. [ ] `pen-editor`: a "dependent update" pass that runs after any command touching a bound
       element, inside the same history entry.
4. [ ] `pen-render`: curved arrows as Catmull-Rom → cubic Bézier; elbow routing on a grid.
5. [ ] Deleting a shape unbinds its arrows; deleting a container deletes its text. *(Text half done.)*

## Tests

- Move a shape 100 px: bound arrow endpoint moves, unbound end stays.
- Undo the move restores both the shape and the arrow in one step.
- Load a file with a dangling binding: it's dropped with a warning, not a crash.

## Notes

- **Bound text (done early).** `Kind::Text` carries `container: Option<Container>`, where
  `Container::Shape(id)` wraps inside a rect/diamond/ellipse and `Container::Path(id)` labels a
  line or arrow. Containers keep no back-reference list: a second copy of the link could
  disagree with the first, and the lookup only runs on commit (O(n), never per frame). The
  variant duplicates the container's kind, which never changes, so rendering can lay text out
  without the document.
- `pen_core::bound::follow` is the dependent-update pass for text. `Editor::execute` runs it on
  every command, so moves, resizes, point edits, deletes and undo keep text attached in one
  history entry. Live previews get the same behaviour from `follow::Followers`. It looks up the
  followers when the preview's membership changes, then re-fits only those texts per move.
- Shape text bounds are the shape's padded inscribed box (`bound::text_box`). The app wraps at
  that width and centres vertically, so a resize needs no font measurement in the editor.
  Typing past the box grows the shape downwards, live in the preview, and the growth is
  committed with the text. Resizing a shape smaller than its text does not yet clamp, so the
  text overflows centred.
- Labels keep their measured size, are centred halfway along the polyline, and paint on a
  canvas-coloured backdrop to mask the line. The backdrop also hides anything else under the
  label; Excalidraw masks only the line.
- Plain double-click on a line now labels it; point editing moved to Ctrl+Enter or
  Ctrl+double-click. Double-click on empty canvas starts free text, as in Excalidraw.
