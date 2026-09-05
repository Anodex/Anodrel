# Anodrel Font Faces

**Status:** Portable character-map, horizontal-metric and pair-kerning,
bounded-outline, and quadratic-path foundation.

`anodrel-font` validates one already-owned TrueType face held in memory and
looks up a Unicode scalar value in its character map. It can also extract one
validated TrueType outline as contours of on- and off-curve points. It
can deterministically convert those contours to exact quadratic paths. It is
the first step toward first-party glyph coverage for Linux and future native
hosts; it does not yet draw text. `docs/TEXT_RUNS.md` describes the separate
bounded glyph-run layer, while `docs/GLYPH_RENDERING.md` describes the renderer
adapter that can flatten one path.

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
let metrics = face.font_metrics()?;
let advance = glyph.map(|glyph| face.horizontal_metric(glyph)).transpose()?;
let kerning = match (face.glyph_id('A'), face.glyph_id('V')) {
    (Some(left), Some(right)) => face.horizontal_kerning(left, right)?,
    _ => 0,
};
let outline = glyph.map(|glyph| face.glyph_outline(glyph)).transpose()?;
let path = outline.as_ref().map(anodrel_font::GlyphOutline::quadratic_path);
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

## Horizontal metrics

Metrics become available only when the face contains a complete `head`,
version-1.0 `maxp`, version-1.0 `hhea`, and `hmtx` table set. Map-only faces
remain valid, but a partial metric set is malformed. `FontMetrics` returns the
validated `unitsPerEm`, ascender, descender, and line gap in signed font design
units. `HorizontalMetric` returns one glyph's advance width and left side
bearing in those same units.

`unitsPerEm` must be in the OpenType range 16 through 16,384. The parser also
requires the four reserved `hhea` values and its metric-data format to be zero,
and validates an exact `hmtx` length from `numGlyphs` and
`numberOfHMetrics`. A glyph after the long metric records reuses the final
advance width but reads its own side bearing, as the table format defines.

Metric lookup reads at most two validated table values and allocates nothing.
It does not infer a missing metric from an outline. See Microsoft's OpenType
[`head`](https://learn.microsoft.com/en-us/typography/opentype/otspec183/head),
[`hhea`](https://learn.microsoft.com/en-us/typography/opentype/spec/hhea), and
[`hmtx`](https://learn.microsoft.com/en-us/typography/opentype/spec/hmtx)
specifications.

## Horizontal pair kerning

An optional conventional OpenType version-0 `kern` table becomes available only
beside a complete validated horizontal-metric source. `horizontal_kerning(left,
right)` validates both face-local glyph IDs, then returns the signed design-unit
adjustment for that ordered pair. It returns a closed metrics-unavailable
outcome for a map-only face. A metric face with no `kern` table, an unmatched
pair, or only valid non-horizontal tables returns zero; it never chooses a
system or fallback text engine.

The parser accepts at most 32 declared subtables and at most 2,097,124 table
bytes. It validates every subtable's range and coverage flags, and selects only
format-0 tables marked horizontal without `minimum` or `cross-stream`
behaviour. Selected tables must have an exact length, valid binary-search
fields, strictly sorted in-range pairs, and only final zero padding. A lookup
binary-searches borrowed font bytes and makes no allocation. Matching selected
values add in table order, except an override subtable replaces the earlier
result. Other valid `kern` formats and modes are ignored rather than guessed.

This is a deliberately small pre-shaping feature, not OpenType layout. GPOS,
class kerning, vertical and cross-stream placement, device variation, and every
other `kern` format remain absent. See [Decision
0208](decisions/0208-first-party-pair-kerning-stays-bounded.md) and Microsoft's
OpenType [`kern`](https://learn.microsoft.com/en-us/typography/opentype/spec/kern)
table specification.

## Glyph outlines

`FontFace::glyph_outline` becomes available only when the face contains one
complete TrueType outline set: `head`, version-1.0 `maxp`, `loca`, and `glyf`.
Their table ranges, glyph count, location format, every ascending location, and
the final glyph-data offset are validated while the face is parsed. A face with
none of these tables remains valid for character-map lookup alone; a partial
set is rejected as a malformed face.

An outline call reads one location pair in constant time, then validates and
decodes a simple glyph or the bounded translated composite subset. It returns
its header bounds plus a flat point list and one contour-end index for each
contour. `point_slice` exposes one contour without copying it. Points use signed
font design units and preserve the TrueType on-curve flag. `quadratic_path`
converts those points into a later rasterizer's line and quadratic segments
without changing the source outline.

The parser expands packed flag runs and relative x/y vectors only after every
read is range-checked. It ignores instruction bytes without executing them,
rejects the reserved simple-glyph flag bit, and bounds one extracted glyph to
4,096 contours and 16,384 points. The `loca` index makes finding a glyph
constant time; extraction allocates only the returned outline and its temporary
flag buffer.

An empty location returns an empty outline with zero bounds. A located simple
glyph with zero contours also returns an empty outline, preserving its validated
header bounds. It may end immediately after the header, or it may carry a
range-checked instruction length and ignored instruction bytes; no instructions
are ever executed. A composite may contain at most 128 translated components
and nest at most 16 levels; cycles and expanded point or contour counts above
the same simple-glyph limits are closed errors. Every result coordinate remains
a signed 16-bit design unit. Component scaling, rotation, shearing, scaled
offsets, and point attachment return closed unsupported outcomes rather than
rounding or silently changing geometry. A nonzero character-map result outside
`maxp` returns a closed invalid-glyph error. Invalid contour endpoints, repeats,
coordinate deltas, instructions, flags, or trailing non-padding bytes return a
closed malformed-outline error. See [Decision 0207](decisions/0207-first-party-composite-glyphs-start-with-bounded-translations.md).

The table and packed-point rules follow Microsoft's OpenType documentation for
[`loca`](https://learn.microsoft.com/en-us/typography/opentype/spec/loca) and
[`glyf`](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf).

## Quadratic paths

`GlyphOutline::quadratic_path` is a pure conversion of one validated
outline. It emits one closed sequence per source contour: each sequence has a
start point and a flat slice of `LineTo` or `QuadraticTo` segments whose final
endpoint is that start point. An empty outline has no contours or segments.

`GlyphPathPoint` stores each coordinate in **doubled design units** as a signed
32-bit integer. A point stored by the font becomes `2 × coordinate`; an implied
point between two adjacent off-curve controls is the average of their doubled
values (equivalently, the exact integer sum of their source coordinates). That
preserves half-unit midpoints without floating-point rounding and leaves later
scaling policy explicit.

The converter follows the TrueType contour rules: consecutive off-curve
controls gain an implied on-curve midpoint; one off-curve control followed by
an explicit on-curve point becomes one quadratic segment; adjacent explicit
on-curve points become a line. When a contour starts off-curve, it starts at
the last explicit on-curve point if there is one, otherwise at the exact
midpoint between the last and first controls. This rule and the packed source
point format are defined by Microsoft's OpenType
[`glyf`](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf)
table documentation.

Conversion costs one bounded linear pass over the existing points. It makes
one flat segment buffer and two small contour-index buffers, rather than one
allocation per contour. No font bytes, OS service, callback, global cache, or
application data are read during conversion.

## Deliberately absent

- font discovery, paths, package policy, fallback, or a default family;
- OpenType layout beyond bounded conventional pair kerning: GPOS, shaping,
  ligatures, class kerning, variation selection, bidirectional text, script
  handling, line breaking, text measurement, and text sizing;
- component transforms and point attachment, hinting, rasterization, colour
  glyphs, bitmap strikes, or a canvas dependency;
- application-controlled font bytes or a protocol field carrying fonts.

A later composite extension may consume the validated `GlyphOutline` or
`GlyphPath` only after a dedicated contract establishes transformation,
attachment, precision, and output bounds. None may turn this parsing crate into
a hidden font loader or a general text engine.

## Verification

The crate has synthetic minimal face tests for format 4 and format 12,
non-BMP lookup, missing glyphs, selection priority, malformed table ranges,
truncated maps, invalid group/segment ordering, and glyph-ID overflow. Simple
outline tests cover short and long location formats, packed repeats and signed
coordinate vectors, contour slices, empty glyphs, bounded translated composite
components, and malformed outline boundaries. Metric tests cover table
completeness, units-per-em bounds, shared advances, individual side bearings,
and malformed table lengths. Quadratic-path tests cover explicit lines,
off-curve controls,
implied half-unit midpoints, off-curve contour starts, closure, and empty
outlines. Those tests contain no machine font and no operating-system
dependency, so they run identically on every supported development host.
Pair-kerning tests cover empty and absent sources, valid accumulated and
override pairs, invalid IDs, binary-search fields, ordering, bounds, and
irrelevant subtable modes.
