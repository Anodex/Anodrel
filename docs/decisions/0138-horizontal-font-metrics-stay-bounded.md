# Decision 0138: Horizontal font metrics stay bounded

- Status: Accepted
- Date: 2026-08-30

## Context

Anodrel can now prepare one simple glyph coverage mask but cannot place the
next glyph or derive a line baseline from the font itself. Those values belong
to the TrueType `head`, `hhea`, and `hmtx` tables, not to a glyph outline.

Making a later renderer infer advances from bounding boxes would overlap glyphs
or add arbitrary gaps. Accepting a malformed metric table would also turn an
untrusted glyph index into an unchecked renderer read.

## Decision

Extend `anodrel-font` with a complete optional horizontal-metric source:
`head`, version-1.0 `maxp`, version-1.0 `hhea`, and `hmtx`. The parser checks
the `head` magic and `unitsPerEm` range, `hhea` reserved values and metric-data
format, a nonzero `numberOfHMetrics` no larger than `numGlyphs`, and the exact
`hmtx` byte length implied by those counts. A partial metric set is malformed.

`FontFace::font_metrics` returns units per em, ascender, descender, and line
gap. `FontFace::horizontal_metric` returns one glyph's advance width and left
side bearing in font design units. Both return a closed unavailable or invalid
glyph outcome. The latter performs at most two bounded table reads and follows
the OpenType rule that glyphs after the long-metric records reuse the final
advance width while retaining individual side bearings.

## Consequences

- A later first-party run builder can choose a baseline and advance individual
  glyphs without consulting an operating-system text API.
- Parsed metric values borrow the caller-owned face bytes and allocate nothing
  per lookup.
- Outline extraction and metrics remain independently optional: map-only faces
  are still valid, while a source can offer metrics before simple outlines.

## Deliberately absent

- text sizing, pixel placement, glyph masks, caching, drawing, source loading,
  kerning, hinting, shaping, fallback, line wrapping, and public text APIs;
- vertical metrics, OS/2 typographic metrics, variation adjustments, bitmap
  metrics, device metrics, and a Linux application host.

## Alternatives considered

**Derive advances from outlines.** A bounding box is not a horizontal advance
and loses the font's explicit side-bearing data. Refused.

**Keep the metric tables opaque until shaping.** The platform needs a small
single-glyph baseline before it can responsibly build a full shaping system.
Refused.

**Use host text metrics.** This would make the owned Linux text path depend on
platform-specific font behavior. Refused for this portable slice.

## Revisit conditions

Revisit before kerning, vertical metrics, OS/2 metrics, variations, composite
glyph positioning, run layout, font caching, a face source, or a public text
capability.
