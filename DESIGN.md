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
| `T_CAPTION` | 11 / 500 | Inspector field captions |
| `T_CONTROL` | 12 / 450 | Inspector control labels |

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
S/M/L/XL row. All three are segmented controls with the toolbar's teal active state. Clicking away
or switching to another drawing tool hides the panel. Document text uses bundled Patrick Hand,
Inter or Roboto Mono; interface labels continue to use Inter.

Outline sloppiness uses a small stroke icon after the toolbar's tools. Clicking cycles
Architect, Artist and Cartoonist for the selection or future shapes. Its tooltip identifies
the current style, keeping outline styling available without an always-open sidebar.

A 48-point document strip sits at the top edge. Tabs are 196 × 32 pt with 8-point corners;
the active tab uses the pale teal wash, inactive labels use muted ink. Each has a close
button and a live unsaved dot; long names fade, with the full path in a tooltip. The strip
scrolls horizontally and brings the active tab into view. A fixed plus button stays accessible
at the right edge. Switching tabs lands immediately, including the canvas and selection.

A compact File / Preferences menu bar sits 16 pt below the document strip, beside the
centred drawing toolbar. Its dropdowns use the same paper surfaces, teal active state and
compact controls as the inspector. File contains New tab, Open, Save, Export and Reset canvas,
with shortcut hints and a divider before Reset. Reset opens a compact confirmation modal. Muted file
status remains at the bottom right in a fixed live-text box so camera updates stay live-only.
An open menu temporarily replaces the shape inspector in the left column, keeping their
surfaces from overlapping. Closing the menu restores the inspector in place with the current
selection. Menus close on Escape, a second press of their tab, or an outside press. Dismissal consumes
the pointer gesture so it cannot draw on the canvas.

## Properties and canvas settings

The left inspector is a compact 216-point floating surface (192-point inner column, 12-point
padding) that hugs its content and scrolls once it reaches the window's height. It is denser
than the toolbar on purpose: the toolbar stays the primary 44-point control, while the
inspector reads as a quiet secondary column. It shows selection properties or drawing
defaults and hides when neither applies. Canvas preferences live in the top-left Preferences
menu. Fields that differ across the selection
say “Mixed”; changing one field preserves every other value.

### Inspector controls

- **Captions** use `T_CAPTION` (11 / 500) in `MUTED`, 6 pt above their control. Sections are
  14 pt apart.
- **Segmented controls** sit in a recessed `ACCENT_SOFT` track, 32 pt tall with 2 pt inset.
  The active segment takes the toolbar's teal state; inactive ones only tint on hover.
  Labels use `T_CONTROL` (12 / 450); arrowhead symbols use the larger `T_GLYPH`.
- **Swatches** are 28-point targets around an 18-point chip. Transparent is the first swatch,
  marked with a red slash. The active swatch gets the teal wash and a 1 pt teal ring.
- **Hex inputs** are flat 32-point fields in the track colour; a teal border marks focus. They
  always show the stored sRGB value and apply on Enter.
- **Opacity** is a 20-point slider with a teal fill and a ringed knob; its value sits at the
  right of the caption. It follows the pointer immediately and commits on release.
- **Preferences** (tool lock, grid snapping, dark mode) are switch rows with the shortcut in
  muted caption type beside the label.

Text controls appear first when relevant. Canvas colour is the last field in Preferences. Dark appearance
reverses colour lightness without modifying the document. Appearance is global across tabs
and persists as an editor preference, separately from canvas files and recovery. The grid has a 20-world-point pitch
and follows camera changes through live scale and phase values.


## Arrange and edit

A round rotation handle sits 28 screen points above the selection bounds, connected to the
outline by a fine teal stem. Single and multi-element selections share the eight resize
handles. Locked selections keep their outline and hide transform handles; the toolbar hint
explains how to unlock them.

The context menu uses native Rune menus where supported. The fallback is a paper surface with
two compact action columns, 30-point rows, shortcut labels in muted badge type, and a fine
vertical divider. Selection and clipboard actions sit on the left; ordering, alignment and
distribution sit on the right. Disabled actions stay muted. The menu clamps within an
8-point viewport margin and dismisses on Escape or an outside click without drawing.
