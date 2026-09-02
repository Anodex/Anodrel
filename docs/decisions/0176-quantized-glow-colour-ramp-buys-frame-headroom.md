# Decision 0176: A quantized glow colour ramp buys bounded frame headroom

**Status:** Accepted

**Date:** 2026-09-02

## Context

The retained glow mask removed repeated alpha sampling and blurring from the
Startup Lab reveal, but each of its two composites still evaluates a three-stop
linear gradient and four-channel interpolation for every covered pixel. That
is the largest sustained cost that remains in the effect. Replacing all
gradients globally would make an unmeasured visual tradeoff in text, panels,
and other surfaces.

## Decision

Add an explicit 512-sample quantized-linear paint to the owned canvas and use
it only for the diffuse Anodrel mark glow. It maps a linear-gradient position
to the nearest precomputed colour rather than re-scanning stops and
interpolating channels at every masked pixel.

The glow retains its exact three stops, axis, opacity, mask, placement, and
compositing order. Its brand test compares every sampled colour with the
straight exact glow paint and limits every RGBA channel to one level of error.
The renderer performance workload continues to report both exact and
quantized mask-fill stages, while the Windows release frame guards remain the
acceptance gate.

## Consequences

- One high-cost diffuse effect gains a constant-time colour lookup per pixel.
- The approximation is local, explicit, and bounded rather than a hidden
  rasterizer-wide quality change.
- Ramp construction uses bounded memory and occurs only when a caller selects
  the explicit quantized paint.
- Exact paints remain the default for semantic content and all existing callers.

The initial optimized performance-lab comparison (11 samples per stage) reduced
the isolated mask-fill stage from 1.779 ms to 1.113 ms. This is stage evidence,
not a whole-frame claim; the release frame guard continues to protect the
actual animation.

## Revisit conditions

Revisit for a different ramp resolution, retained colour ramps, a colour-space
change, another approximation, GPU composition, or a measurement showing that
the lookup no longer improves sustained frame cost.
