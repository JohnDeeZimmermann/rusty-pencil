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

Save with Ctrl+S, Save as with Ctrl+Shift+S, and open with Ctrl+O. Scene files are versioned
JSON with the `.drusty-pen` extension; dropping one on the window opens it. Autosave keeps a
separate recovery scene in your platform's local application data directory and restores it
on the next launch. Reset canvas asks for confirmation; save a named copy before resetting
if you want to retain the current work.

For isolated visual checks, use `cargo run -p drusty-pen -- --snap snaps/m4.png --scale 1
--script app/drusty-pen/scripts/m4-text-style.json --at 18.6`. Snapshots do not read or write
normal recovery. Add `--data-dir PATH` to test recovery in a separate directory, or `--open
PATH` to load a scene directly.
