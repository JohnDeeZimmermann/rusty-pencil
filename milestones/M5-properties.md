# M5: Properties panel and app settings

**Goal:** style elements the way Excalidraw does, and theme the app.

**Estimate:** 1–2 days.

## Done when

- [ ] A side panel shows properties for the selection (or the active tool's defaults).
- [ ] Stroke and background colour (palette, hex input, transparent).
- [ ] Fill: hachure, cross-hatch, solid. Stroke width: thin, bold, extra bold. Stroke style:
      solid, dashed, dotted. Edges: sharp, round. Opacity slider.
- [ ] Arrowheads at start and end: none, arrow, bar, dot, triangle, circle, diamond (filled and
      outline).
- [ ] New elements use the last chosen properties. Tool lock (`Q`) keeps the tool after drawing.
- [ ] Dark mode (Alt+Shift+D), canvas background colour, grid (Ctrl+') with snap to grid.

## Tasks

1. [ ] `pen-core`: complete `Props`; mixed values across a multi-selection show as "mixed".
2. [ ] `pen-editor`: `AppState` (defaults, theme, grid, tool lock); persisted in the scene file.
3. [ ] `pen-render`: hachure / cross-hatch fill lines, dash and dot patterns as polyline segments.
4. [ ] App: panel built from rune-kit components, one file per property group.
5. [ ] Dark mode renders the scene with inverted luminance like Excalidraw, without changing
       stored colours.

## Tests

- Changing a property on three selected elements is one undo step.
- Dash pattern lengths are constant in world units at every zoom.

## Notes
