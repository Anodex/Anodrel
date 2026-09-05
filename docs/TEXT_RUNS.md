# Anodrel Text Runs

**Status:** Portable first-party single-line placement foundation.

`anodrel-text` converts one UTF-8 value into a bounded ordered run of glyph
identifiers and horizontal pen positions from one already-validated
`anodrel-font::FontFace`. It is the narrow middle layer between owned font
parsing and a future host-owned glyph painter. It does not load a font, draw a
glyph, choose a size, or present a window.

## Boundary

The public entry point is deliberately direct:

```rust
let run = anodrel_text::TextRun::build(&face, "Anodrel")?;
```

The result owns a vector of `RunGlyph` values. Each value holds the validated
font-local glyph identifier, its horizontal pen position, and its advance in
font design units. `TextRun` also carries the face-wide metrics required by a
later caller to choose a baseline and line height.

The run borrows neither the input string nor the face bytes after construction.
It exposes no source text, font bytes, file path, cache, native handle, or
operating-system state.

## Placement rule

For every Unicode scalar in source order, the builder asks the selected
character map for one nonzero glyph identifier, reads that glyph's horizontal
metric, records the current pen position, and then advances it:

~~~text
glyph.pen_x = pen_x
pen_x += glyph.advance_width
~~~

All values remain in unscaled font design units. A later renderer selects its
own explicit pixel scale and baseline, then uses the existing glyph adapter to
convert an individual outline to canvas coverage. This keeps line placement out
of font parsing and keeps device coordinates out of the run builder.

## Limits and closed outcomes

One run accepts at most **1,024 Unicode scalars** and at most **1,048,576**
total horizontal design units. The scalar count bounds the one owned result
vector; the advance bound keeps a later device transform inside a practical
numeric envelope. The builder returns no partial run when either limit would be
exceeded.

It also returns no partial run when the face has no complete horizontal metrics,
when one scalar has no nonzero mapped glyph, or when a mapped glyph lies outside
the metric source. An empty value is a valid zero-glyph run, provided the face
has complete horizontal metrics.

## Deliberately absent

- font discovery, paths, a default typeface, application-selected face bytes,
  fallback, caching, or a protocol operation;
- glyph outlines, glyph masks, paint, native drawing, device density, and
  host-window integration;
- kerning, ligatures, contextual shaping, combining-mark placement,
  bidirectional ordering, script handling, line breaking, wrapping, or text
  editing.

This is not a claim that scalar-to-glyph placement is correct typography for
every script. It is an explicit first-party foundation for the simple
single-line text that Anodrel's current native surfaces use. More complete text
behaviour requires a separate contract rather than silently widening this one.

## Verification

Synthetic in-memory TrueType faces prove source order, exact pen advances,
empty runs, unavailable glyph and metric outcomes, scalar and advance limits,
and rejection of a character map that points outside its metric source. No test
opens a machine font or calls an operating-system API.

## Related material

- [Font faces](FONTS.md)
- [Glyph rendering](GLYPH_RENDERING.md)
- [Renderer](RENDERER.md)
- [Decision 0204](decisions/0204-first-party-text-runs-stay-unshaped-and-bounded.md)
