# drusty-pen design language

**Quiet, precise, approachable.** A pale studio surface, small floating controls, and a muted
teal selection. Personality comes from the drawings and carefully spaced shapes. Chrome stays
light enough to leave attention on the canvas.

Tokens live in `app/drusty-pen/src/design.rs`. Views use those tokens for colours, sizes,
radii, typography, and motion.

## Principles

1. **Light surfaces.** White islands have a 1 pt neutral border and a soft, low-opacity shadow.
2. **Restrained colour.** A pale teal wash and deep teal icon identify the active tool. Hover
   uses a neutral wash; shortcut labels use secondary text colour.
3. **Clear hierarchy.** The toolbar is the primary control. Hints are plain secondary text,
   without a surrounding box. Zoom is a compact surface anchored at the bottom left.
4. **Comfortable targets.** Keep 44 pt buttons, with precise 20 pt icons and modest corners.
5. **Subtle feedback.** A press scales to 0.98. State changes use rune's springs and respect
   its Reduce Motion setting.

## Colour (sRGB)

| Token | Hex | Use |
|---|---|---|
| `INK` | `#1e1e1e` | Icons, primary text, default canvas stroke |
| `PAPER` | `#ffffff` | Floating surfaces |
| `CANVAS` | `#f8f9f7` | Pale neutral canvas |
| `ACCENT` | `#ddeee8` | Active tool fill |
| `ON_ACCENT` | `#235e4d` | Active tool icon and shortcut |
| `ACCENT_SOFT` | `#f0f3f1` | Inactive tool hover |
| `MUTED` | `#707874` | Hints and inactive shortcuts |
| `BORDER` | `#dde2de` | Floating surface borders |
| `SHADOW` | Ink at 18/255 alpha | Soft elevation |

## Shape and spacing

- Surface border: 1 pt. Shadow: 2 pt down, 8 pt blur.
- Toolbar radius: 12 pt. Button and zoom radius: 8 pt.
- Button: 44 × 44 pt. Icon: 20 × 20 pt, 1.7 pt stroke.
- Toolbar padding: 6 pt. Button gap: 4 pt. Hint gap: 12 pt.
- Chrome sits 16 pt from window edges.
- Shortcuts sit inside each button, inset 3 pt, without badges.

## Type

Use rune's bundled interface font. Keep controls compact and avoid heavy weights.

| Token | Size / weight | Use |
|---|---|---|
| `T_HINT` | 12 / 450 | Unboxed hint |
| `T_LABEL` | 12 / 500 | Zoom label |
| `T_BADGE` | 9 / 500 | Plain shortcut digits |

## Icons and canvas

Tool icons use `pen_render::draw`, so their geometry matches the drawing tools. The document's
stroke colours and shape defaults remain independent of chrome colours. Committed and draft shapes use the same seeded hand-drawn contours; toolbar icons stay clean.

## Selection feedback

Selection outlines use `ON_ACCENT` at 1 screen point, inset from white 8 × 8 screen-point
handles with 2-point corners. Geometry has a 5-screen-point gap to the selection bounds.
Marquee fill uses `SELECTION_WASH` (the accent ink at 24/255 alpha). These dimensions stay
constant through pan and zoom. Canvas feedback appears and disappears immediately; toolbar
press and hover motion remains subtle.

## Text and file controls

The type inspector appears only while the text tool is active or text is selected or being
edited. Font family and alignment use compact outline icons with tooltips; size uses a single
S/M/L/XL row. Controls keep 44-point targets and the toolbar's teal active state. Clicking away
or switching to another drawing tool hides the panel. Document text uses bundled Patrick Hand,
Inter or Roboto Mono; interface labels continue to use Inter.

Outline sloppiness uses a small stroke icon after the toolbar's tools. Clicking cycles
Architect, Artist and Cartoonist for the selection or future shapes. Its tooltip identifies
the current style, keeping outline styling available without an always-open sidebar.

File actions sit at the bottom right, with a muted status above them. Reset opens a compact
modal with Cancel and Reset canvas actions. The status uses a fixed live-text box so camera
updates can remain live-only.

## Properties and canvas settings

The left inspector is a 260-point floating surface with 44-point controls. It shows selection
properties or drawing defaults, followed by canvas settings. A bounded inner column scrolls
without panning the canvas. Fields that differ across the selection say “Mixed”; changing
one field preserves every other value. Palette swatches preview the current appearance;
hex inputs always show the stored sRGB value and apply on Enter. Transparent is explicit.

Text controls appear first when relevant. Fill, stroke, edge and arrowhead choices use the
same teal active state as the toolbar. Opacity follows the pointer immediately and commits
on release. Canvas settings expose tool lock, grid snapping, dark mode and background colour.
Dark appearance reverses colour lightness without modifying the document. The grid has a
20-world-point pitch and follows camera changes through live scale and phase values.
