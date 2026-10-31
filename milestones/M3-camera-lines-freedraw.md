# M3: Camera, lines, arrows, freedraw

**Goal:** move around an infinite canvas and draw lines, arrows and freehand strokes.

**Estimate:** 1 day.

## Done when

- [ ] Wheel pans; Ctrl+wheel and pinch zoom about the cursor; Space+drag, middle-drag and the
      hand tool (`H`) pan.
- [ ] Ctrl+`+`/`-`, Ctrl+0 (reset), Shift+1 (fit all), Shift+2 (fit selection); zoom % shown.
- [ ] "Scroll back to content" button appears when nothing is visible.
- [ ] Line (`L`) and Arrow (`A`): drag for two points, or click point to point and finish with
      Enter/Esc/double-click. Shift snaps to 15° steps.
- [ ] Double-click a line/arrow (or Ctrl+Enter) to edit points: move, add, delete.
- [ ] Freedraw (`P`): smooth, variable-width strokes.

## Tasks

1. [x] `pen-editor`: `Camera { scroll, zoom }` with `screen_to_world`, `world_to_screen`,
       `zoom_about(screen_pt, factor)`, zoom clamped to 10%–3000%. Done in M0 (camera A).
2. [ ] `pen-core`: `Kind::{Line, Arrow, Freedraw}` storing points relative to the element origin.
3. [ ] `pen-editor`: multi-point line tool state machine and the point editor.
4. [ ] `pen-render`: freedraw outline from points (port of the perfect-freehand algorithm, or an
       evaluated crate), simplified on commit.
5. [ ] `pen-render`: arrowhead geometry (default arrow head only; other heads in M5).
6. [ ] App: wire `pointer_buttons([Middle])`, Space in `on_key` / `on_key_up`, the hand tool
       and zoom shortcuts. `on_wheel` and `on_magnify` (live-only) already exist from M0.

## Tests

- Camera: `screen_to_world(world_to_screen(p)) == p`; zoom about a point keeps that point fixed.
- Line tool: click-click-Enter gives three points; Esc mid-draw keeps placed points.
- Freedraw: 1 000 input points → outline is closed and has no NaN.

## Notes
