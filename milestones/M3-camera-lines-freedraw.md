# M3: Camera, lines, arrows, freedraw

**Goal:** move around an infinite canvas and draw lines, arrows and freehand strokes.

**Estimate:** 1 day.

## Done when

- [x] Wheel pans; Ctrl+wheel and pinch zoom about the cursor; Space+drag, middle-drag and the
      hand tool (`H`) pan.
- [x] Ctrl+`+`/`-`, Ctrl+0 (reset), Shift+1 (fit all), Shift+2 (fit selection); zoom % shown.
- [x] "Scroll back to content" button appears when nothing is visible.
- [x] Line (`L`) and Arrow (`A`): drag for two points, or click point to point and finish with
      Enter/Esc/double-click. Shift snaps to 15° steps.
- [x] Double-click a line/arrow (or Ctrl+Enter) to edit points: move, add, delete.
- [x] Freedraw (`P`): smooth, variable-width strokes.

## Tasks

1. [x] `pen-editor`: `Camera { scroll, zoom }` with `screen_to_world`, `world_to_screen`,
       `zoom_about(screen_pt, factor)`, zoom clamped to 10%–3000%. Done in M0 (camera A).
2. [x] `pen-core`: `Kind::{Line, Arrow, Freedraw}` storing points relative to the element origin.
3. [x] `pen-editor`: multi-point line tool state machine and the point editor.
4. [x] `pen-render`: freedraw outline from points (port of the perfect-freehand algorithm, or an
       evaluated crate), simplified on commit.
5. [x] `pen-render`: arrowhead geometry (default arrow head only; other heads in M5).
6. [x] App: wire `pointer_buttons([Middle])`, Space in `on_key` / `on_key_up`, the hand tool
       and zoom shortcuts. `on_wheel` and `on_magnify` (live-only) already exist from M0.

## Tests

- Camera: `screen_to_world(world_to_screen(p)) == p`; zoom about a point keeps that point fixed.
- Line tool: click-click-Enter gives three points; Esc mid-draw keeps placed points.
- Freedraw: 1 000 input points → outline is closed and has no NaN.

## Notes

- Line and arrow samples are relative to an element origin. Horizontal/vertical paths and
  pen dots remain selectable and transform without requiring an area.
- Enter includes the live endpoint; Escape and double-click retain placed vertices only.
  Focus cancellation and tool changes discard the unfinished path. Backspace removes the
  last placed vertex. Each finished path and each point edit is one undo step.
- Point editing shows vertex and midpoint handles at constant screen sizes. Click any
  segment to insert a point; Delete removes the active point while retaining two endpoints.
- Evaluated `perfect_freehand` 0.1.1 (MIT, no dependencies) and pinned it in `pen-render`:
  it replaces streamline, pressure-outline, join and cap geometry. Rune currently exposes
  pointer positions without stylus pressure, so input spacing supplies simulated pressure.
  Pressure is stored per sample before commit-time simplification. The adapter preserves
  pressure for two-sample strokes, where the port otherwise synthesizes 0.5-pressure samples.
  Quadratic midpoint interpolation flattens the outline at the existing 0.02-world-point
  tolerance. Repeated samples and single-point dots are covered.
- Return-to-content visibility uses a bounds hierarchy refreshed after commands and history
  changes. Camera events query it without scanning the document or rebuilding scene paint.
  The return button uses live opacity (zero is also unhittable); zoom and selection overlays
  use live values. Hand/Space panning does not take keyboard focus on every pointer press.
- The world transform stays a zero-size node, but **paint layers need real bounds**. Rune
  culls canvases by their layout rect even when their shapes overflow it. Layers now rebase
  paint to local coordinates and carry world-space layout bounds, so zooming away from the
  world origin does not hide visible content. `instant_changes` on the world keeps layout
  animation out of document edits. A native-runtime test checks emitted primitives at 3000%.
- Visual fixture: `app/drusty-pen/scripts/m3-paths.json`, snapshots at 7.1 s (paths, moved and
  inserted points), 8.5 s (200%), 9.6 s (3000%), 10.5 s (return button), 11.5 s (fit all).
  Run with `cargo run -p drusty-pen -- --snap snaps/m3.png --scale 1 --script
  app/drusty-pen/scripts/m3-paths.json --at 7.1`.

- Release check on this machine (`--scale 1`, scripted input): 427 frames through 7.1 s,
  frame CPU mean **0.338 ms**, p95 **0.747 ms**. At 3000% through 9.6 s: mean **0.416 ms**,
  p95 **0.702 ms**. These measure the small visual fixture, not large-document throughput.
  All **124 workspace tests** pass, including GUI-neutral tool events, 1,000-sample pen
  outlines, native Rune dispatch, and live-only camera/overlay/button updates.
