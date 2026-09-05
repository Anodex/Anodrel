# Anodrel Glyph Mask Cache

**Status:** Portable bounded coverage-retention foundation.

`anodrel-glyph::GlyphMaskCache` retains coverage masks for one already-validated
`FontFace`. It sits after outline extraction and path flattening, but before a
host composites coverage into a canvas. It does not load a font, choose text,
draw a canvas, or provide a process-wide cache.

## Ownership and key

One cache borrows exactly one face for its lifetime. It therefore cannot reuse a
glyph identifier from a different face: glyph ID `7` has meaning only within the
face that constructed that cache.

Each retained mask is keyed by three exact values:

- face-local glyph identifier;
- `f32` pixels-per-design-unit bit pattern;
- the exact fractional x and y phases of the baseline.

The integer baseline translation is not part of coverage. `mask_at` splits a
validated baseline into an integer offset and a fractional phase; callers can
reuse the returned local mask and apply only its integer offsets at composite
time. There is no phase quantization, so the cache never silently changes
anti-aliasing quality to increase its hit rate.

## Resource limits

The existing glyph converter rejects a single coverage mask above 262,144 pixels
(one MiB of `f32` coverage). This cache retains at most **64 masks** and at most
**2,097,152 pixels** (eight MiB) across all entries. An access updates a
monotonic local recency order. On insertion, least-recently-used entries are
removed until both limits fit. A valid mask too large for the total cache budget
is returned to the caller but is not retained.

All storage belongs to the cache value. Dropping it drops all masks. No global,
thread-local, shared, persistent, application, or operating-system cache exists.
There is no cache lookup or statistics operation in the application protocol.

## Error and placement behaviour

`mask_at` returns no partial value when the baseline is non-finite or outside
the documented canvas range, when a glyph outline is unavailable or unsupported,
or when the existing renderer refuses its complexity or scale. A cache miss
rasterizes only after those checks. The caller receives a `CachedGlyphMask`
containing a borrowed-viewable mask and the signed integer translation to apply;
it never receives face bytes, paths, a cache key, or a native object.

## Deliberately absent

- a default font, paths, font discovery, source loading, fallback, application
  font selection, or a public protocol operation;
- text-run construction, shaping, kerning, layout, clipping policy, paint, and
  native window integration;
- sharing between faces, threads, processes, or application sessions;
- cache sizing knobs, eviction callbacks, cache readback, or a background
  cleanup task.

The cache does not replace the current Windows GDI text route. One fixed,
non-windowed Windows probe now proves selected-face source, run-to-glyph
placement, and offset composition twice without a copy. A later visible host
integration must still define retained source ownership, clipping, invalidation,
quality/performance parity, and accessibility agreement before it can make that
claim.

## Verification

Synthetic faces prove that identical glyph, scale, and phase reuse one retained
entry even at different integer locations; changed phases do not reuse coverage;
the fixed entry bound evicts; invalid baseline and excessive-render paths retain
nothing. The tests use no machine font or operating-system API.

## Related material

- [Font faces](FONTS.md)
- [Text runs](TEXT_RUNS.md)
- [Glyph rendering](GLYPH_RENDERING.md)
- [Decision 0205](decisions/0205-glyph-mask-caches-stay-face-local-and-bounded.md)
- [Decision 0206](decisions/0206-mask-offset-compositing-does-not-copy-coverage.md)
