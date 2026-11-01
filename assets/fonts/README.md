# Bundled fonts

## Families and licences

All fonts are unmodified TrueType fonts from the Google Fonts repository, distributed
under SIL Open Font License 1.1. Each font's original copyright and full licence are included
beside it. They are embedded in the application, so drawing does not require a network.

| File | Family | Upstream | Licence |
|---|---|---|---|
| `PatrickHand.ttf` | Hand-drawn | [Patrick Hand](https://github.com/google/fonts/tree/main/ofl/patrickhand) | `patrickhand-OFL.txt` |
| `Inter.ttf` | Normal and interface | [Inter](https://github.com/google/fonts/tree/main/ofl/inter) | `inter-OFL.txt` |
| `RobotoMono.ttf` | Code | [Roboto Mono](https://github.com/google/fonts/tree/main/ofl/robotomono) | `robotomono-OFL.txt` |

## Updating

Keep the matching upstream licence when replacing a font. The app's measurement engine and
Rune runtime must load the same files and family names. Recheck multiline alignment, caret
position, IME composition and saved bounds after a font update. Rune uses system fallback
when a writing system is absent from the bundled fonts.
