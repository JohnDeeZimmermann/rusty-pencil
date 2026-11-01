# Bundled fonts

## Families and licences

All fonts are TrueType fonts from the Google Fonts repository, distributed
under SIL Open Font License 1.1. Each font's original copyright and full licence are included
beside it. They are embedded in the application, so drawing does not require a network.

| File | Family | Upstream | Licence |
|---|---|---|---|
| `PatrickHand.ttf` | Hand-drawn | [Patrick Hand](https://github.com/google/fonts/tree/main/ofl/patrickhand) | `patrickhand-OFL.txt` |
| `Inter.ttf` | Normal and interface | [Inter](https://github.com/google/fonts/tree/main/ofl/inter) | `inter-OFL.txt` |
| `RobotoMono.ttf` | Code | [Roboto Mono](https://github.com/google/fonts/tree/main/ofl/robotomono) | `robotomono-OFL.txt` |
| `Inter-web.ttf` | Normal and interface (browser build) | Subset of `Inter.ttf` | `inter-OFL.txt` |

## The browser subset

The browser downloads every embedded byte and has no system fonts to fall back on, so the web
build embeds `Inter-web.ttf` instead of `Inter.ttf`. It keeps Inter's Latin, Greek, Cyrillic,
punctuation, arrows, maths, box-drawing, technical and dingbat ranges and its core layout
features, and drops stylistic sets and hinting. That saves about 145 KB gzipped. Every glyph the
interface itself uses must be in the subset; regenerate it after updating `Inter.ttf`:

```sh
pyftsubset assets/fonts/Inter.ttf --no-hinting --output-file=assets/fonts/Inter-web.ttf \
  --layout-features='kern,liga,calt,ccmp,locl,mark,mkmk,tnum' \
  --unicodes='U+0000-036F,U+0370-03FF,U+0400-04FF,U+1E00-1EFF,U+2000-206F,U+20A0-20CF,U+2100-214F,U+2190-23FF,U+2500-27BF'
```

## Updating

Keep the matching upstream licence when replacing a font. The app's measurement engine and
Rune runtime must load the same files and family names. Recheck multiline alignment, caret
position, IME composition and saved bounds after a font update. Rune uses system fallback
when a writing system is absent from the bundled fonts on the desktop; the browser has none,
so missing glyphs show as boxes there. Draw interface symbols as icons rather than relying on a
fallback font.
