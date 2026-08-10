# Decision 0064: Retained raster effects may trade a bounded, tested error for frame cost

**Status:** Accepted

**Date:** 2026-08-09

## Context

Anodrel composes every first-party surface in software. There is no GPU path and
no platform compositor to hand an effect to, so a blur costs real milliseconds
on the thread that is also running the message loop.

Measuring the Startup Lab's reveal stage by stage showed where those
milliseconds go. Of an 7.9 ms frame, the Anodrel mark accounted for about 5.4 ms
and its glow for about 4.4 ms of that: 1.0 ms sampling the artwork's alpha into
a coverage mask, 1.0 ms blurring it, and 2.3 ms compositing the result twice
through a gradient. The glow was rebuilt from scratch on every frame.

What it was rebuilt *for* is the striking part. Across a full reveal the glow
mask stayed 365 or 366 pixels square while the mark's own width moved by 1.5
pixels in total — a fraction of a pixel per frame — and the blur applied to it
quantizes its radius to a 16-pixel box, so the blur itself was bit-identical on
every one of those frames. Two milliseconds a frame were being spent to express
a sub-pixel difference in something about to be spread over tens of pixels.

Caching keyed on the exact request cannot recover that. Measured over a real
reveal, 43 glow requests produced 42 distinct exact keys: the sub-pixel origin
and the requested radius change every frame even when nothing that survives the
blur does.

## Decision

A retained raster effect may be keyed on the quantities that visibly determine
its result rather than on the exact request, provided the resulting error is
**bounded, measured, and asserted by a test**.

The mark's glow is the first such effect. Its mask is keyed on the source
raster's bucket, the mark's size rounded to whole pixels, the padding, and the
box radius the blur actually applies — not the requested radius. A retained mask
is repositioned to whole pixels and composited without being rebuilt or copied.

The error this admits is a placement of up to half a pixel in each axis.
`a_retained_glow_matches_one_built_in_place` composites both paths over the
backdrop they are drawn on and holds them to 8 levels out of 255 across
quarter-pixel placements. Measured, they agree exactly when the mark lands on
the pixel grid and differ by at most 7 at the half-pixel worst case.

Two supporting pieces make that key honest rather than a guess:

- `Mask::blur_box_radius` reports the radius the blur will actually use, so a
  caller keys on the blur that will happen instead of restating the formula.
- `Mask::reposition` moves a retained mask in place, because a mark-sized glow
  is around half a megabyte of coverage that copying would duplicate per frame.

## Consequences

The reveal's mean frame cost fell from 7.9 ms to 6.7 ms and its most expensive
sustained frame from 10.0 ms to 8.0 ms, against a 16 ms interval — measured by
the guards described in `docs/PERFORMANCE.md`.

Every effect retained under this decision owes a test that bounds its error
against a straightforwardly correct implementation, kept spelled out at the test
rather than shared with the code under test. Without that test the decision does
not apply: an approximation nobody has measured is a defect, not a trade.

This does not license approximation in the parts of the renderer that carry
meaning. Text, the mark's own artwork, and layout geometry stay exact. What may
be approximated is a diffuse effect whose own filter already discards more
detail than the approximation introduces.

The cache is per-thread and bounded to three masks, cleared rather than evicted
when it overflows, matching the scaled-raster cache beside it. Surfaces that
draw the mark at many sizes in one frame will miss more often than they hit;
none does today, and the guards would show the cost if one started.

## Alternatives considered

**Key on the exact request.** Correct and useless: 42 distinct keys in 43
requests.

**Quantize the animation instead.** Snapping the mark's scale to whole pixels
would make an exact cache hit, at the price of a reveal that steps rather than
glides. The animation is the thing being paid for; the glow's sub-pixel
placement is not.

**Build the glow at bucket resolution and scale it when compositing.** This is
what the mark's body already does, and it would cache across sizes rather than
only within one. It needs a scaled mask composite the canvas does not have, and
it introduces resampling error on top of placement error. Worth revisiting if a
surface ever needs the mark glowing at several sizes at once.

**Leave it.** The frame fits the interval today on the reference machine either
way. It fits with a third less room, on hardware that is not the slowest this
platform intends to run on, for work that changes nothing anybody can see.
