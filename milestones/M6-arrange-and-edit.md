# M6: Arrange and edit

**Goal:** the editing actions people use daily.

**Estimate:** 2 days.

## Done when

- [ ] Rotate handle; Shift snaps to 15°. Multi-element resize and rotate.
- [ ] Flip horizontal / vertical (Shift+H / Shift+V).
- [ ] Duplicate (Ctrl+D) and Alt+drag duplicate. Copy, cut, paste; pasting plain text makes a
      text element. Copy / paste styles (Ctrl+Alt+C / V).
- [ ] Group / ungroup (Ctrl+G / Ctrl+Shift+G), nested; double-click enters a group.
- [ ] Lock / unlock elements.
- [ ] Z-order: send backward / to back, bring forward / to front.
- [ ] Align (6 ways) and distribute (2 ways).
- [ ] Eraser (`E`); Alt restores what the current stroke erased.
- [ ] Right-click context menu with these actions.

## Tasks

1. [ ] `pen-core`: `angle` on every element; hit testing and bounds honour rotation.
2. [ ] `pen-core`: `group_ids: Vec<GroupId>` (outermost last), `locked: bool`.
3. [ ] `pen-core`: fractional-index z-keys so reordering touches only moved elements.
4. [ ] `pen-editor`: an `Action` registry (id, label, shortcut, `enabled(&state)`, `run`) shared by
       shortcuts, context menu and later the command palette.
5. [ ] App: clipboard via rune's clipboard API with our own MIME-tagged JSON plus plain text.
6. [ ] App: context menu via `rune::PopupMenu`.

## Tests

- Rotate 90° then hit-test a corner point.
- Group, then click a member selects the whole group; double-click selects only the member.
- Every `Action` is undoable in one step.

## Notes
