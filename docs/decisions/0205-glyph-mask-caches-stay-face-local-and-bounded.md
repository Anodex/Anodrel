# Decision 0205: Glyph mask caches stay face-local and bounded

- Status: Accepted
- Date: 2026-09-05

## Context

The owned glyph adapter can make one bounded coverage mask, but repeating that
work for a static glyph on every frame would discard the advantage of retained
software rendering. A global cache would create unbounded lifetime and make a
glyph ID from one font accidentally meaningful for another. Caching by only
glyph ID and scale would also reuse different subpixel anti-aliasing phases.

## Decision

Add `GlyphMaskCache` to `anodrel-glyph`. It borrows one validated `FontFace` and
retains at most 64 masks and 2,097,152 coverage pixels using a local
least-recently-used order. Its key contains one face-local glyph ID, exact scale
bits, and exact fractional baseline phases. The integer baseline translation is
returned separately so movement by whole pixels reuses the same coverage.

The cache has no configuration, global state, callback, protocol operation, or
operating-system dependency. A mask that is valid but cannot fit the cache's
total budget is returned without retention. Dropping the cache drops every
retained mask.

## Consequences

- A future Windows or Linux renderer can avoid repeated outline conversion and
  coverage rasterization without sharing a face or unbounded allocation.
- Fractional placement remains visually exact rather than being quantized to a
  cache-friendly approximation.
- Cache ownership and clipping remain a later host concern; this is not a
  Windows text-painter replacement.

## Deliberately absent

- a face source, text layout, outline fallback, paint operation, window
  integration, cache metrics, tuning surface, or cross-thread sharing;
- application-selected fonts, public cache access, background eviction, disk
  persistence, or any protocol change.

## Alternatives considered

**One global cache.** It hides lifetime and face identity, risks memory growth,
and would need locking on a draw path. Refused.

**Key only glyph and scale.** Different fractional baselines have different
coverage at their edges. Refused as a visual correctness defect.

**Quantize subpixel phase.** It improves hit rate by accepting a visible
placement error. Refused until a measured host need can justify an explicit
quality bound.

## Revisit conditions

Revisit before a host connects this cache to painting, adds a face source,
permits cache sizing, shares across threads, or defines shaping and clipping.
