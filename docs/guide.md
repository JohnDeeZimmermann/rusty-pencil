# The rusty-pencil guide

Everything rusty-pencil can do, one workflow at a time. For the quick tour with pictures, start
at the [README](../README.md).

## Run and draw

Run `cargo run -p rusty-pencil` with the pinned Rust toolchain. Use the toolbar or letter shortcuts
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

Use **+**, File → New tab, Ctrl+T or Ctrl+N to open another canvas. Each tab keeps its own
undo history, selection, camera, tools and canvas settings. Appearance stays global. Click a
tab to switch; Ctrl+Tab and
Ctrl+Shift+Tab cycle through them. Ctrl+1…8 selects a tab by position, and Ctrl+9 selects the
last tab. Use Cmd for command shortcuts on macOS (Ctrl+Tab also works there).

The top-left File menu contains Open, Save, Export and Reset canvas. Save the active tab with
Ctrl+S, use Ctrl+Shift+S for Save as, and open with Ctrl+O. Opening or dropping scene files
creates tabs; opening a file already in a tab focuses it. Scene files are versioned JSON
with the `.rusty-pencil` extension. A dot beside a tab name means it has changes to save to a
file. Closing with **×** or Ctrl+W offers Save, Discard and Cancel when needed. Closing the
last tab leaves a fresh canvas.

Autosave keeps a separate recovery scene for each tab and an atomic workspace manifest in
your platform's local application data directory. On launch, open tabs, their file paths and
the active tab are restored, including unsaved work. Recovery does not overwrite named
files. Reset canvas only clears the active tab and asks for confirmation first.

In the browser build, tab shortcuts use Alt (Alt+T, Alt+W, Alt+PageDown, Alt+1…9) because the
browser keeps the Ctrl versions, saving downloads a copy, and recovery lives in browser storage.
See [web.md](web.md).

## Style and view

The left inspector changes selected elements or the defaults for the next drawing. Choose
stroke and background palettes, enter a six-digit hex colour and press Enter, or choose
Transparent. Fill, stroke width and pattern, edges, opacity and arrowheads appear in the
scrolling inspector. A mixed field says “Mixed”; each change updates the entire selection in
one undo step. An opacity drag previews immediately and commits when released.

The top-left Preferences menu contains Keep tool, Grid & snap, Dark mode and Canvas colour.
Drawing returns to selection. Press Q (or Keep tool) to keep drawing with the same tool.
Toggle dark mode with Alt+Shift+D and the grid with Ctrl+'. The grid snaps to 20 world points;
canvas settings also change the background colour. Drawing defaults and canvas settings
travel with saved scenes and autosave recovery. Light/dark mode is an editor-wide preference
stored separately in `preferences.json`: switching tabs, opening files or resetting a canvas
keeps the same appearance. Changing appearance does not mark a canvas as unsaved.

## Visual checks

The README screenshots come from the scenes in `assets/readme/scenes/`; open one with
`--open` and snap it the same way.

For isolated checks, use `cargo run -p rusty-pencil -- --snap snaps/m5.png --scale 1
--script app/rusty-pencil/scripts/m5-properties.json --at 19.5`. Substitute `m5-dark.json` for dark
mode and grid coverage. Snapshots do not read or write normal recovery. Add `--data-dir PATH`
to test recovery in a separate directory, or `--open PATH` to load a scene directly.
Use `tabs.json` under the same script directory with `--at 6.5` to check three
independent canvases and the tab bar.

## Images and export

Press 9 or the image button to insert a PNG/JPEG, paste an image, or drop image files on
canvas. Select an image to trim any edge by 10% or reset its crop; cropping keeps the source
pixels and supports undo. Images are embedded once per content hash in saved scenes.

File → Export (Ctrl+Shift+E) saves PNG or SVG at 1×/2×/3× with background, dark-mode and
selection-only choices. Copy the selection as PNG with Ctrl+Shift+C, or as SVG source with
Ctrl+Alt+Shift+C. SVG text is outlined using the bundled fonts. Exported images do not embed
an editable scene; save a `.rusty-pencil` file to keep editing later.

Open or drop `.excalidraw` files to import supported elements. A notice counts skipped
unsupported types. Individual images are limited to 16 MiB / 16 megapixels; decoded previews
are limited to 64 sources / 128 MiB, with placeholders beyond that imported-scene budget.

## Frames and your library

Press F and drag to create a frame. Select its edge or title, edit its name in the right
panel and press Enter. Frames clip their children; dragging an element's centre into or
out of a frame changes its ownership immediately. Moving a frame moves its children, while
resizing changes the clipping window. Selecting a frame before Ctrl+Shift+E exports just
that frame at its exact size. Frames are axis-aligned and cannot be nested.

Open **Library** on the right and use **Add selection** to keep reusable drawings. Drag a
thumbnail onto the canvas, or click it to place a copy in the viewport centre. Import and
export `.rusty-pencil-library` files to move your collection between machines. The library stays
in the platform's local app-data directory, alongside workspace recovery, and embeds its
images. Placing an item creates fresh ids and one undo step.

## Find, inspect and present

Use Ctrl+F to find text or frame names, Ctrl+/ for the command palette and `?` for shortcut
help. The palette includes arrangement actions, every tool and view modes; type to filter,
use arrow keys to move through results and press Enter. The secondary F / S / K / I controls
on the right expose frames, lasso, laser and eye-dropper.

Lasso selection uses S; hold Shift to add to an existing selection. The K laser leaves a
short fading trail without changing the drawing. Press I and click to sample a colour from
a shape, image or canvas; Shift applies the sampled colour to the fill. Alt+S enables object
snapping with alignment guides; holding Ctrl temporarily bypasses it during a drag.

Preferences and keyboard shortcuts provide Zen (Alt+Z), read-only View (Alt+R), and Stats
(Alt+/). Stats shows an element's id and editable x/y/width/height in world points, plus its
angle in degrees. Select one element, change a field and press Enter to apply an undoable edit.

Select an element and press Ctrl+K to add an HTTP(S) link or a local `#element-id` destination.
An empty value removes the link. Use **Open link** or Ctrl+click the element to follow it;
web links open in your system browser and local links focus the destination on the canvas.

For M9 visual coverage, run the snapshot command with
`--script app/rusty-pencil/scripts/m9-frames.json --at 8.4` to inspect dark-mode frames and
the personal library. Other review times are recorded in the M9 milestone file.

The app was previously named `drusty-pen`. Existing `.drusty-pen` scenes and
`.drusty-library` libraries still open. Recovery, preferences, and the local library keep
using the original data directory and internal filenames so existing work remains available.
