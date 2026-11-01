# M5: Properties panel and app settings

**Goal:** style elements the way Excalidraw does, and theme the app.

**Estimate:** 1–2 days.

## Done when

- [x] A side panel shows properties for the selection (or the active tool's defaults).
- [x] Stroke and background colour (palette, hex input, transparent).
- [x] Fill: hachure, cross-hatch, solid. Stroke width: thin, bold, extra bold. Stroke style:
      solid, dashed, dotted. Edges: sharp, round. Opacity slider.
- [x] Arrowheads at start and end: none, arrow, bar, dot, triangle, circle, diamond (filled and
      outline).
- [x] New elements use the last chosen properties. Tool lock (`Q`) keeps the tool after drawing.
- [x] Dark mode (Alt+Shift+D), canvas background colour, grid (Ctrl+') with snap to grid.

## Tasks

1. [x] `pen-core`: complete `Props`; mixed values across a multi-selection show as "mixed".
2. [x] `pen-editor`: `AppState` (defaults, theme, grid, tool lock); persisted in the scene file.
3. [x] `pen-render`: hachure / cross-hatch fill lines, dash and dot patterns as polyline segments.
4. [x] App: panel built from rune-kit components, one file per property group.
5. [x] Dark mode renders the scene with inverted luminance like Excalidraw, without changing
       stored colours.

## Tests

- Changing a property on three selected elements is one undo step.
- Dash pattern lengths are constant in world units at every zoom.

## Notes

The inspector uses rune-kit fields, buttons and text inputs on the existing design tokens.
It scrolls independently of the canvas. Text controls remain first when relevant; each new
property group lives in its own module. Hex inputs accept six sRGB digits and apply on Enter.
Mixed fields keep their distinct values until that particular field is edited.

Settings are owned by the editor and serialized through the existing `AppState` file envelope
in `pen-core` (re-exported by `pen-editor`), avoiding a core-to-editor dependency. Older scenes
supply defaults for the added fields. Theme changes invert HSL lightness at presentation time;
scene memo keys include the appearance, while cached world geometry and stored colours stay
unchanged. Pattern dimensions and the 20-point grid are in world units.

An opacity drag previews selected elements and commits one grouped command on release;
cancelling restores the originals. Drawing returns to selection unless Q locks the tool.
Grid snapping applies to shape corners, line/arrow vertices, selection moves and resizes,
point drags and keyboard nudges. Freedraw samples retain their natural path.

Visual checks: `scripts/m5-properties.json` and `scripts/m5-dark.json` in the app, captured at
19.5 seconds with scale 1. Both cover all three fills, dashes, dots, start/end markers and the
inspector; the dark variant also enables the grid.

Release visual run (19.5 seconds, 1171 frames, scale 1): light mean frame CPU 0.666 ms,
p95 1.318 ms; dark with grid mean 0.646 ms, p95 1.245 ms. These are local headless
measurements of the scripted scene, not a large-document benchmark.
