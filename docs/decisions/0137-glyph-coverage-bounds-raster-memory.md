# Decision 0137: Glyph coverage bounds raster memory

- Status: Accepted
- Date: 2026-08-30

## Context

`anodrel-glyph` can now turn a bounded TrueType path into a bounded canvas
polygon, but a canvas polygon may cover an arbitrarily large pixel rectangle.
The generic canvas mask is intentionally useful for large artwork and therefore
does not impose a glyph-specific allocation budget.

Allowing a transformed font outline directly into that generic allocation would
make one glyph's memory and raster work depend on its untrusted coordinate
extent and caller-selected scale.

## Decision

Add a checked `Mask::for_path_bounded` canvas constructor. It rejects non-finite
geometry or padding, checked bounds outside signed canvas coordinates, and a
pixel area above the supplied limit before allocating coverage. The existing
unbounded `Mask::for_path` remains unchanged for authored artwork.

`anodrel-glyph::coverage_mask` first creates the already-bounded flattened
canvas path and then calls that checked constructor with a fixed 262,144-pixel
limit. It returns the normal owned `Mask` on success or the existing closed
`GlyphRenderError::TooComplex` outcome. It does not cache a mask or expose an
allocation setting to application code.

## Consequences

- One glyph coverage result has a maximum one-megabyte `f32` backing buffer.
- Coverage bounds are checked before the software rasterizer iterates a pixel.
- The canvas keeps a general mask API while text retains a separate, explicit
  resource envelope.
- A future cache can retain only already-bounded coverage values.

## Deliberately absent

- glyph caching, repositioning policy, paint/composite calls, metrics, shaping,
  font loading, source identity, and text layout;
- configurable glyph memory, multi-glyph runs, GPU work, Linux host integration,
  or a public application text capability.

## Alternatives considered

**Add one global mask limit.** Authored backgrounds and effects have different
budgets from glyphs, so this would constrain unrelated canvas callers. Refused.

**Check only after mask allocation.** The allocation itself is the risk.
Refused.

**Let a future cache evict oversized masks.** A cache cannot undo the first
unbounded allocation or raster pass. Refused.

## Revisit conditions

Revisit before cache keys, retained glyph runs, configurable quality, direct
scan conversion, composite glyphs, or a public text API.
