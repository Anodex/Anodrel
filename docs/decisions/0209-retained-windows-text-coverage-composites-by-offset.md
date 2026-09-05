# Decision 0209: Retained Windows text coverage composites by offset

- Status: Accepted
- Date: 2026-09-05

## Context

The Windows host asks GDI to shape and rasterize a text run once, then retains
the resulting antialiased coverage in its bounded per-thread run cache. The
existing draw path rebuilt a `Mask` by cloning every retained coverage buffer on
each repaint solely to set its canvas origin. A Startup Lab reveal repaints the
same labels many times while their paint or position changes, so that allocation
and copy consume frame time without changing a pixel.

`Canvas::fill_mask_offset` already defines a safe immutable-mask composition
route: it applies a signed integer translation during one paint call using wide
intermediates and clipped source ranges.

## Decision

Store each cached Windows GDI run as one origin-zero `Mask`, its baseline
insets, and advance. On every draw, retain the mask's coverage unchanged and
pass the calculated whole-pixel origin only as `fill_mask_offset` arguments.
The current 512-entry cache, GDI ownership, text shaping, hinting, paints,
alignment, and layout measurements remain unchanged.

## Consequences

- Repainting a cached text run no longer allocates or copies its coverage just
  to move it.
- A mask remains immutable shared cache data; one draw cannot change another
  draw's origin or coverage.
- The GDI path remains the Windows text source. This does not claim that the
  portable font, glyph, and text-run foundations have replaced it.

## Deliberately absent

- a new font source, shaping engine, text layout model, cache scope or size,
  fractional placement, transform stack, clipping state, or application text
  capability; and
- changes to GDI font selection, native handles, accessibility semantics, or
  presentation order.

## Alternatives considered

**Clone and reposition the retained mask for every draw.** Correct output, but
the coverage allocation and copy are proportional to rendered text area on each
repaint. Refused.

**Mutate the cached mask's origin.** Avoids the copy but makes independent
draws interfere and breaks cache immutability. Refused.

**Add a general canvas transform stack.** Larger stateful graphics surface than
this single, already-supported integer placement needs. Deferred.

## Revisit conditions

Revisit before fractional text placement, native text-source replacement,
retained transform state, host-level font policy, or cross-platform text
integration.
