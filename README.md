<img src="assets/icon.svg" width="64" height="64" alt="">

# drusty-pen

Created in [T3 Code](https://t3.codes).

## Run and draw

Run `cargo run -p drusty-pen` with the pinned Rust toolchain. Use the toolbar or letter shortcuts
for shapes, paths, pen, selection and text (`T`). Click to place text, press Enter for another
line, and finish with Escape or a click away. Double-click a text element to edit it again.
Double-click inside a rectangle, diamond or ellipse to write text that wraps inside it (the
shape grows to fit), or on a line or arrow to label it. That text moves, resizes and deletes
with its shape. Ctrl+Enter or Ctrl+double-click edits a line's points.
Text controls appear when creating, editing or selecting text. Use their icons to change
font family and alignment, or S/M/L/XL for size. The stroke icon at the end of the toolbar
cycles outline sloppiness. Undo uses Ctrl+Z
(Cmd+Z on macOS).

## Keep your canvas

The top-left File menu contains Open, Save and Reset canvas. Save with Ctrl+S, Save as with
Ctrl+Shift+S, and open with Ctrl+O. Scene files are versioned
JSON with the `.drusty-pen` extension; dropping one on the window opens it. Autosave keeps a
separate recovery scene in your platform's local application data directory and restores it
on the next launch. Reset canvas asks for confirmation; save a named copy before resetting
if you want to retain the current work.

## Style and view

The left inspector changes selected elements or the defaults for the next drawing. Choose
stroke and background palettes, enter a six-digit hex colour and press Enter, or choose
Transparent. Fill, stroke width and pattern, edges, opacity and arrowheads appear in the
scrolling inspector. A mixed field says “Mixed”; each change updates the entire selection in
one undo step. An opacity drag previews immediately and commits when released.

The top-left Preferences menu contains Keep tool, Grid & snap, Dark mode and Canvas colour.
Drawing returns to selection. Press Q (or Keep tool) to keep drawing with the same tool.
Toggle dark mode with Alt+Shift+D and the grid with Ctrl+'. The grid snaps to 20 world points;
canvas settings also change the background colour. Defaults and settings travel with saved
scenes and autosave recovery.

## Visual checks

For isolated checks, use `cargo run -p drusty-pen -- --snap snaps/m5.png --scale 1
--script app/drusty-pen/scripts/m5-properties.json --at 19.5`. Substitute `m5-dark.json` for dark
mode and grid coverage. Snapshots do not read or write normal recovery. Add `--data-dir PATH`
to test recovery in a separate directory, or `--open PATH` to load a scene directly.
