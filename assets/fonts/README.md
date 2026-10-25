# assets/fonts

## Purpose
The code font of the editor: JetBrains Mono 2.304 Regular and its licence, the SIL Open Font
License 1.1 (OFL-1.1). Both are embedded in every build that links `va-ui` (architecture §9.6,
m4-editor-shell.md §5.5, §8.4). The files are third-party data, copied unmodified: never edit them.

## Contents
| Entry | Type | Purpose |
|-------|------|---------|
| `JetBrainsMono-Regular.ttf` | font (binary) | JetBrains Mono 2.304 Regular, from `fonts/ttf/` in the release archive. 273,900 bytes, SHA-256 `a0bf60ef0f83c5ed4d7a75d45838548b1f6873372dfac88f71804491898d138f` |
| `OFL.txt` | licence text | The OFL 1.1 text with the JetBrains Mono copyright line, from the root of the archive. 4,399 bytes, SHA-256 `30f0c136e3c88e422d0791acd97238870f9054a9729bc34cf2ff0d4ed8cac4ad` |

## Entry points
- `crates/va-ui/src/fonts.rs`: `EMBEDDED_CODE_FONT` (`include_bytes!` of the TTF) and
  `EMBEDDED_FONT_LICENSE` (`include_str!` of `OFL.txt`), so every binary carries the licence with
  the font. `font_definitions` makes the TTF the `va-code` family, after a valid custom
  `editor.font_file` and before egui's monospace and emoji fonts (m4 §5.5); it is a `todo!()` stub
  until WP-E implements it in wave 2.

**Provenance.**
- Source: `https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip`,
  downloaded on 2026-10-24 with the user's approval (m4 §8.4). The approval covers that file from
  that URL only.
- Archive `JetBrainsMono-2.304.zip`: 5,622,857 bytes, SHA-256
  `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf`. It was extracted into its
  own empty folder outside the repository, and only the two members above were copied out; the
  archive itself is not in the repository.
- A new font version is a new download: it goes back to the Architect and the user, then this
  table, `DEPENDANCE.md` and `IMPORT.md` change in the same commit.

**Licence (OFL-1.1).** The font may be used, embedded and redistributed with the software,
including commercially, provided the copyright line and the licence text travel with it; it may
not be sold by itself, and a modified version stays under the OFL (`OFL.txt` declares no reserved
font name). VisualAssembler embeds it unmodified,
embeds `OFL.txt` with it, and M8 packaging also copies `OFL.txt` next to the executable (not
implemented yet). The font is not a crate, so cargo-deny never sees it: `DEPENDANCE.md` and
`IMPORT.md` record it.

## Dependencies
- Internal: embedded by `crates/va-ui/src/fonts.rs`.
- External: none (data only).

## Tests / usage
```bash
sha256sum assets/fonts/JetBrainsMono-Regular.ttf assets/fonts/OFL.txt
```
It must print the two hashes of the table. `.gitattributes` marks `*.ttf` as binary, so git never
converts it. The planned `crates/va-ui/tests/fonts.rs` (WP-E) checks that `font_definitions`
registers the embedded font first.
