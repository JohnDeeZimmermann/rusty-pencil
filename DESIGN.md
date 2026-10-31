# drusty-pen design language

**Bold, playful, hand-made.** The chrome looks like it was cut from card and outlined with the
same pen you draw with: solid ink outlines, hard offset shadows, one loud accent, springy
motion. The canvas stays calm so drawings stay the star; the chrome has the personality.

Tokens live in `app/drusty-pen/src/design.rs`. Never hard-code a colour, size or radius in a
view: add a token or use an existing one.

## Principles

1. **Ink outlines everything.** Islands and pills have a 2 pt ink border. No hairlines, no grey
   borders.
2. **Hard shadows, not blur.** Raised chrome casts a solid ink shadow 3 pt straight down, like a
   sticker on paper. Pressed chrome sinks into it.
3. **One loud accent.** Violet marks "this is on": the active tool, focus, selection (M2).
   Everything else is ink, paper or grey.
4. **Chunky and round.** Big hit targets (44 pt), generous radii, thick icon strokes.
5. **Springy.** Every press squashes and springs back. State changes animate on rune's springs,
   never instantly and never with linear fades.

## Colour (sRGB)

| Token | Hex | Use |
|---|---|---|
| `INK` | `#1e1e1e` | Outlines, shadows, icons, text. Also the default stroke on canvas. |
| `PAPER` | `#ffffff` | Island and pill fill. |
| `CANVAS` | `#fdfcf8` | The canvas: a warm off-white, so white islands lift off it. |
| `ACCENT` | `#7c4dff` | Active tool fill, focus, selection. |
| `ON_ACCENT` | `#ffffff` | Icons and text on the accent. |
| `ACCENT_SOFT` | `#ece5ff` | Hover wash on inactive buttons. |
| `MUTED` | `#6b6b75` | Secondary text (hints). |
| `BADGE` | `#ffd84d` | Shortcut badges: a sunny pop against the violet. |

## Shape

| Token | Value |
|---|---|
| `OUTLINE` | 2 pt ink border |
| `LIFT` | 3 pt hard ink shadow, no blur |
| `R_ISLAND` | 16 pt |
| `R_BUTTON` | 12 pt |
| `R_PILL` | fully round (height / 2) |

## Size and spacing

4 pt grid. `S1` 4, `S3` 12, `S4` 16, `S6` 24.

- Tool button: 44 × 44, icon 22 × 22, icon stroke 2.2 pt.
- Island padding `S1` + 2, gap `S1`.
- Chrome sits `S4` from the window edges.

## Type

Inter (rune's default). Chrome text is never lighter than 500.

| Token | Size / weight | Use |
|---|---|---|
| `T_HINT` | 13 / 500 | Hint pill |
| `T_LABEL` | 13 / 650 | Zoom pill, buttons with text |
| `T_BADGE` | 9 / 750 | Shortcut digit badges |

## Motion

- Press: `press_scale(0.9)` on buttons, `0.96` on islands' children with text.
- Colour and shadow changes ride rune's springs; don't call `Tx::instant` for chrome.
- Respect Reduce Motion: rune turns press scale off; don't add motion that bypasses it.

## Icons

Tool icons are the shapes themselves, drawn by `pen_render::draw` with the icon stroke, so the
toolbar and the canvas always match. Inactive icons are `INK`, active ones `ON_ACCENT`.

## Canvas

The canvas follows Excalidraw's defaults (ink `#1e1e1e`, 2 pt strokes, round edges) until the
hand-drawn style in M4. Chrome personality never leaks into the drawing itself.
