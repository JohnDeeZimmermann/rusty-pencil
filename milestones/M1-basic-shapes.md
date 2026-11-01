# M1: Basic shapes

**Goal:** pick a tool and drag to draw rectangles, diamonds and ellipses that stay on the canvas.

**Estimate:** 2–3 h.

## Done when

- [x] Toolbar with Rectangle, Diamond, Ellipse; keys `R`/`2`, `D`/`3`, `O`/`4` switch tools.
- [x] Dragging draws a live preview; releasing commits the element.
- [x] Shift keeps a square/circle; Alt draws from the centre.
- [x] A zero-size click creates nothing.

## Tasks

1. [x] `pen-core`: `ElementId` (newtype over `u64`, monotonic per document), `Element { id, version,
       seed, props, kind }` (z = index, see Notes), `Kind::{Rect, Diamond, Ellipse}` with a world-space bounding box.
2. [x] `pen-core`: `Document` storing elements in z-order with id lookup; `Command::{Add, Remove}`
       and `Document::apply(Command) -> Result<Command, DocError>` returning the inverse.
3. [x] `pen-core`: geometry types `WorldPt`, `WorldRect` (normalised: `min <= max`).
4. [x] `pen-editor`: `InputEvent::{PointerDown, PointerMove, PointerUp, Key}` and a `Tool` trait or
       enum; shape tool state machine `Idle → Dragging → commit`.
5. [x] `pen-render`: `DrawList` of `Stroke`/`Fill` primitives; build per element, cache by
       `(id, version)`. Clean geometry only (hand-drawn comes in M4).
6. [x] App: convert `DrawList` → `rune::Shape`; two layers: committed scene (memo on document
       revision) and live preview.
7. [x] App: toolbar row with key shortcuts (custom buttons, not rune-kit; see Notes).

## Tests

- Editor: drive each tool with `InputEvent`s, assert the resulting element's kind and bounds,
  including Shift and Alt.
- Core: `apply` then apply the inverse gives back an equal document.

## Notes

- **Z-order is the element's index in `Document`**, not a field on `Element`. Storing both
  could disagree. Revisit if collaboration-style fractional indices are ever needed.
- **Id lookup is linear** (`Document::get`). Fine at this scale; index it once a profile says
  so.
- **`Edges::Round` is the default**, as in Excalidraw: the adaptive radius is a quarter of the
  shorter side, capped at 32 pt. Corners are quadratic curves, as Excalidraw draws them. The
  property UI comes in M5.
- **pen-render flattens curves itself**, to 0.02 world pt (0.6 px at the 3000% maximum zoom),
  so paint is camera-independent and cached by `(id, version)` in `SceneCache`. Rune draws
  strokes as one capsule per segment, with round joins.
- **Two layers under the world node:** `scene` is wrapped in `memo(revision)`, so a drag's
  per-move view rebuild reuses it; `draft` is rebuilt on every move. Both are zero-size canvases
  at the origin, like the world node.
- **Modifiers:** `tx.mods()` is read on every drag phase. `App::modifier_keys()` delivers a
  Shift/Alt press with no pointer movement as `InputEvent::Modifiers`, so the draft squares the
  moment Shift goes down. Some Linux window managers take Alt+drag for moving windows.
- **No selection tool until M2**, so the shape tool stays active after drawing (Excalidraw
  switches back to selection unless the tool is locked).
- **Toolbar is drawn by the app, not rune-kit:** the icons are the shapes themselves, drawn by
  `pen_render::draw`, so toolbar and canvas match exactly.
- Ctrl+wheel zoom (M3) landed early: the M0 hint already promised it.
- Headless check: `cargo run -p rusty-pencil -- --snap out.png --scale 2 --script
  app/rusty-pencil/scripts/shapes.json --at 2.0` draws all three shapes and leaves a draft
  mid-drag.
