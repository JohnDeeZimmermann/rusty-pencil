# M1: Basic shapes

**Goal:** pick a tool and drag to draw rectangles, diamonds and ellipses that stay on the canvas.

**Estimate:** 2–3 h.

## Done when

- [ ] Toolbar with Rectangle, Diamond, Ellipse; keys `R`/`2`, `D`/`3`, `O`/`4` switch tools.
- [ ] Dragging draws a live preview; releasing commits the element.
- [ ] Shift keeps a square/circle; Alt draws from the centre.
- [ ] A zero-size click creates nothing.

## Tasks

1. [ ] `pen-core`: `ElementId` (newtype over `u64`, monotonic per document), `Element { id, version,
       seed, z, props, kind }`, `Kind::{Rect, Diamond, Ellipse}` with a world-space bounding box.
2. [ ] `pen-core`: `Document` storing elements in z-order with id lookup; `Command::{Add, Remove}`
       and `Document::apply(Command) -> Command` returning the inverse.
3. [ ] `pen-core`: geometry types `WorldPt`, `WorldRect` (normalised: `min <= max`).
4. [ ] `pen-editor`: `InputEvent::{PointerDown, PointerMove, PointerUp, Key}` and a `Tool` trait or
       enum; shape tool state machine `Idle → Dragging → commit`.
5. [ ] `pen-render`: `DrawList` of `Stroke`/`Fill` primitives; build per element, cache by
       `(id, version)`. Clean geometry only (hand-drawn comes in M4).
6. [ ] App: convert `DrawList` → `rune::Shape`; two layers: committed scene (memo on document
       revision) and live preview.
7. [ ] App: toolbar row with rune-kit buttons and key shortcuts.

## Tests

- Editor: drive each tool with `InputEvent`s, assert the resulting element's kind and bounds,
  including Shift and Alt.
- Core: `apply` then apply the inverse gives back an equal document.

## Notes
