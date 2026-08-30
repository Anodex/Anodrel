# Anodrel Font Faces

**Status:** First portable parsing foundation.

`anodrel-font` validates one already-owned TrueType face held in memory and
looks up a Unicode scalar value in its character map. It is the first step
toward first-party glyph coverage for Linux and future native hosts; it does
not yet draw text.

## Boundary

The crate accepts bytes only through `FontFace::parse`. It does not open a
path, enumerate installed fonts, read environment variables, cache global
state, call an operating-system API, allocate while looking up a glyph, or
choose a fallback font. A future host remains responsible for obtaining and
owning a particular font before it constructs a face.

The public surface is deliberately small:

```rust
let face = anodrel_font::FontFace::parse(bytes)?;
let glyph = face.glyph_id('A');
```

`glyph` is `Some(GlyphId)` only when the selected character map resolves to a
nonzero glyph. `None` means that the face has no glyph for that character; it
does not select a substitute face or expose a `.notdef` glyph as content.
`GlyphId::value` is available only to a later, host-owned outline and raster
adapter.

## Accepted face and map

The initial slice accepts a big-endian SFNT whose version is the TrueType
`0x0001_0000` value. It validates the bounded table directory and requires one
`cmap` table wholly contained in the supplied byte slice. OpenType CFF,
collections, variation fonts, and every file container are deliberately
outside this contract.

From `cmap`, it accepts these Unicode mappings in this order:

1. Windows platform 3, encoding 10, format 12;
2. Unicode platform 0, encoding 4, format 12;
3. Windows platform 3, encoding 1, format 4;
4. Unicode platform 0, encoding 3, format 4.

Format 12 covers Unicode beyond the Basic Multilingual Plane; format 4 covers
the Basic Multilingual Plane. The parser rejects malformed offsets, lengths,
ordered ranges, overflowing glyph calculations, duplicate required tables, or
unsupported selected maps. It never trusts a font offset merely because it was
present in a directory record. The format choices follow the OpenType `cmap`
definition published by Microsoft: <https://learn.microsoft.com/en-us/typography/opentype/spec/cmap>.

The parser stores byte-slice views into the caller's face. Parsing costs one
bounded directory and map validation pass. A lookup uses binary search over
format-4 segments or format-12 groups, so it is logarithmic in the selected
map and makes no allocation.

## Deliberately absent

- font discovery, paths, package policy, fallback, or a default family;
- OpenType layout: shaping, ligatures, kerning, variation selection, bidirectional
  text, script handling, line breaking, and text measurement;
- glyph outlines, hinting, rasterization, colour glyphs, bitmap strikes, or a
  canvas integration;
- application-controlled font bytes or a protocol field carrying fonts.

A later owned outline reader may consume the `GlyphId` only after a dedicated
contract establishes the face source, glyph geometry, memory limits, and
rasterization quality. It must not turn this parsing crate into a hidden font
loader or a general text engine.

## Verification

The crate has synthetic minimal face tests for format 4 and format 12,
non-BMP lookup, missing glyphs, selection priority, malformed table ranges,
truncated maps, invalid group/segment ordering, and glyph-ID overflow. Those
tests contain no machine font and no operating-system dependency, so they run
identically on every supported development host.
