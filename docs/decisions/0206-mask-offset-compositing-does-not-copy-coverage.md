# Decision 0206: Mask-offset compositing does not copy coverage

- Status: Accepted
- Date: 2026-09-05

## Context

`Mask::positioned` returns an owned copy with a new origin. That is appropriate
when a caller needs an independently mutable mask, but moving a retained glyph
by whole pixels should not duplicate up to one MiB of coverage just to change
two coordinates. The new face-local glyph cache needs a safe way to apply its
integer baseline translation at composite time.

## Decision

Add `Canvas::fill_mask_offset(mask, offset_x, offset_y, paint)`. It reads the
same immutable `Mask` coverage as `fill_mask` and applies one signed integer
translation only for that draw. `fill_mask` delegates with a zero offset. The
operation uses wide intermediate arithmetic and clamps the source rows and
columns before it enters its pixel loop, so extreme public offsets cannot
overflow or write outside the canvas.

## Consequences

- A retained glyph mask can be painted at new whole-pixel positions without a
  coverage allocation or copy.
- Translation is explicit to one paint call and does not mutate the mask or
  become persistent canvas state.
- Fractional placement stays the glyph cache's exact key; this operation never
  resamples, rounds, or changes anti-aliasing coverage.

## Deliberately absent

- fractional translation, scaling, rotation, clipping state, mask mutation,
  caching, retained canvas transforms, or a public application operation;
- host text integration, source-font selection, layout, and draw invalidation.

## Alternatives considered

**Call `Mask::positioned` for every draw.** Correct but duplicates retained
coverage on every movement. Refused for cached glyph composition.

**Mutate a shared mask origin.** It makes one consumer's position affect
another and violates the cache's immutable reuse boundary. Refused.

**Add a general canvas transform stack.** Larger stateful graphics surface than
one bounded translation needs. Deferred.

## Revisit conditions

Revisit before fractional transforms, retained clipping, scaling, rotation, or
host-level cached text painting.
