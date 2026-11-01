# M7: Arrow binding and bound text

**Goal:** diagrams that stay connected.

**Estimate:** 2 days.

## Done when

- [x] Arrow ends near a shape highlight it and bind on release; Ctrl disables binding.
- [x] Moving, resizing or rotating a shape updates bound arrows.
- [x] Select a placed arrow and drag either endpoint to bind or rebind it; Ctrl detaches.
- [x] Arrow type: sharp, curved, elbow. Elbow arrows route orthogonally around their endpoints'
      shapes.
- [x] Double-click a shape to type text inside it; text wraps to the container; vertical align
      top / middle / bottom. *(Shape text shipped early; M7 adds vertical alignment.)*
- [x] Double-click an arrow to add a label that follows the arrow.
- [x] "Wrap text in container" action.

## Tasks

1. [x] `pen-core`: `Binding { element: ElementId, focus: f32, gap: f32 }` on arrow ends; shapes keep
       a back-reference list. An invariant check (debug builds) verifies both sides agree.
2. [x] `pen-core`: `container_id` on text; containers list bound text. *(See notes: only the
       text stores the link.)*
3. [x] `pen-editor`: a "dependent update" pass that runs after any command touching a bound
       element, inside the same history entry.
4. [x] `pen-render`: curved arrows as Catmull-Rom → cubic Bézier; elbow routing on a grid.
5. [x] Deleting a shape unbinds its arrows; deleting a container deletes its text.

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

- **Arrow bindings.** Each element has two optional arrow endpoint links; only arrows use
  them. `focus` is the local radial attachment angle divided by π, and `gap` is world points.
  Rotation therefore preserves the chosen attachment. `Document::bound_arrows` is a derived
  back-reference index rebuilt atomically after commands, with debug checks in both directions;
  it is not serialized as a second source of truth.
- **Dependent order.** `binding::follow` runs before `bound::follow`, so a changed shape updates
  its arrows and then their labels in one history entry. Preview membership is looked up once
  per document revision/selection change; pointer moves fit only affected arrows and text.
  Endpoint point editing also previews binding and reacts immediately to Ctrl. Clipboard and
  Alt-drag duplicates remap internal links and drop links to shapes outside the copied set.
- **Geometry.** Sharp arrows retain their vertices. Curved arrows interpolate them with
  Catmull–Rom cubic Béziers flattened at the shared world tolerance; painting, hit testing,
  arrowheads and label anchors share that curve. Head size uses curve length, while direction
  uses its end tangent. Elbows use a deterministic visibility grid around the endpoint shapes'
  padded world bounds, with orthogonal escape segments and shortest Manhattan routes. Only
  endpoint shapes are obstacles; unrelated scene elements do not affect routing. Overlapping
  endpoint regions may have no collision-free route, in which case a deterministic dogleg is
  retained.
- **Text polish.** Top/middle/bottom alignment uses the same layout for display and editing.
  The context menu's “Wrap text in container” creates a padded rectangle around selected free
  text and binds it atomically. Loading reports the count of dropped invalid/dangling links;
  geometry stays visible and the scene remains editable.
- **Verification.** Event-driven core/editor and Rune tests cover binding feedback, Ctrl,
  live moves, resize/rotation, dependency chains, deletion, duplication, undo, text wrapping
  and alignment; file tests cover recovery and round trips. Scripted screenshots:
  `app/rusty-pencil/scripts/m7-connections.json` (connected sharp/curved/elbow geometry and label)
  `m7-bound-text.json` (wrapping and shape text), and `m7-vertical-align.json` (all vertical
  placements). Release scripted rendering: frame CPU p50 0.437 ms, p95 1.131 ms across 877
  forced frames; pan/zoom still only updates live presentation values.

- **Rebinding follow-up.** Selected arrows expose endpoint handles directly, without a point-edit
  shortcut. Dragging one previews its new target and commits on release; Ctrl detaches, Escape
  cancels, and undo restores the prior link. The body still moves the arrow and the rotation
  handle remains available. `m7-rebind.json` exercises this through real app input.

- [x] **Multiselect performance follow-up.** Rigid previews retain cached contours, Rune
  shapes and laid-out text; only live world offsets change after the first drag frame. Arrows
  that deform still repaint individually in document z-order. Hatch phase follows its shape
  so releasing a move preserves the pattern. Rune input tests assert no view commits during
  connected multiselect motion, immediate subpixel presentation, cancellation and one-step
  undo; renderer tests cover deformation, every element kind and hatch continuity.
  Release timing harness: `cargo test --release -p rusty-pencil multiselect_drag_timing --
  --ignored --nocapture` (7 elements, 360 pointer+frame samples). Before: mean 0.775 ms,
  p95 0.844 ms; after: mean 0.214 ms, p95 0.239 ms. These are headless CPU timings, excluding
  GPU work. `app/rusty-pencil/scripts/multiselect-drag.json` reproduces the connected selection
  and holds a six-second drag for screenshots and rendered traces.
