# M6: Arrange and edit

**Goal:** the editing actions people use daily.

**Estimate:** 2 days.

## Done when

- [x] Rotate handle; Shift snaps to 15°. Multi-element resize and rotate.
- [x] Flip horizontal / vertical (Shift+H / Shift+V).
- [x] Duplicate (Ctrl+D) and Alt+drag duplicate. Copy, cut, paste; pasting plain text makes a
      text element. Copy / paste styles (Ctrl+Alt+C / V).
- [x] Group / ungroup (Ctrl+G / Ctrl+Shift+G), nested; double-click enters a group.
- [x] Lock / unlock elements.
- [x] Z-order: send backward / to back, bring forward / to front.
- [x] Align (6 ways) and distribute (2 ways).
- [x] Eraser (`E`); Alt restores what the current stroke erased.
- [x] Right-click context menu with these actions.

## Tasks

1. [x] `pen-core`: `angle` on every element; hit testing and bounds honour rotation.
2. [x] `pen-core`: `group_ids: Vec<GroupId>` (outermost last), `locked: bool`.
3. [x] `pen-core`: fractional-index z-keys so reordering touches only moved elements.
4. [x] `pen-editor`: an `Action` registry (id, label, shortcut, `enabled(&state)`, `run`) shared by
       shortcuts, context menu and later the command palette.
5. [x] App: clipboard via rune's clipboard API with our own MIME-tagged JSON plus plain text.
6. [x] App: context menu via `rune::PopupMenu`.

## Tests

- Rotate 90° then hit-test a corner point.
- Group, then click a member selects the whole group; double-click selects only the member.
- Every `Action` is undoable in one step.

## Notes

Angles are clockwise radians about each element's unrotated geometry centre. Bounds are
rotation-aware, including tight ellipse and path bounds. Selection frames remain axis-aligned;
rotation and multi-resize preview only touched elements. Shift snaps single-element rotation
to absolute 15° increments. Alt can be pressed or released during a move to toggle duplication
on that same input event; cancelled copies never enter the document. Rotated line-point edits
compensate for changes to the rotation pivot, keeping untouched points fixed in world space.

Groups store their innermost id first. Double-click enters successive nested groups; Escape
leaves the scope. Alignment and distribution treat selected groups as units. Locks follow
bound text, and right-click can select locked content to unlock it. Eraser strokes interpolate
between pointer events, preview removals without changing the document, and commit once;
Alt restores only elements removed by that stroke. A stroke-local bounds hierarchy limits
pointer hit testing to the swept area; unchanged erase membership does not repaint the scene.

Z-keys are variable-length lexicographic fractional indices. Reordering patches only moved
keys, including bound text. Geometry-only patches do not sort or reindex the document. Older
scene files default the new fields and receive keys in their existing paint order.

The action registry owns ids, labels, chords, enablement and execution. Clipboard JSON is
tagged `application/vnd.rusty-pencil.elements+json` inside a lossless HTML `<pre>` transport,
beside readable plain text: the pinned Rune clipboard has no arbitrary MIME slots. Imported
JSON uses the scene validator, and paste remaps element ids, nested groups and text bindings.
Text style paste uses the app's actual font measurements and stays one undo step.

Rune's `PopupMenu` is used where supported (macOS); Linux/Windows use a two-column fallback
with the same registry and design tokens. Scripted headless app tests exercise the fallback,
clipboard transport, styles and input routing. Core/editor tests cover arrange actions and
single-step undo/redo, nested group entry, rotation, modifier changes, locks and erasing.

Visual checks: `app/rusty-pencil/scripts/m6-arrange.json` at 7.0 seconds,
`m6-menu.json` at 7.7 seconds, and `m6-dark.json` / `m6-eraser.json` at 8.3 seconds, scale 1. Rotated text editing
was also inspected in a screenshot. Release arrange run (421 frames): mean frame CPU
0.486 ms, p95 0.883 ms. Release context-menu run (463 frames): mean 0.482 ms,
p95 0.881 ms. These are local scripted scenes, not large-document benchmarks.
