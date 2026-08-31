# Decision 0136: Glyph flattening is bounded and device-explicit

- Status: Accepted
- Date: 2026-08-30

## Context

`anodrel-font` now supplies exact simple TrueType quadratic paths, while
`anodrel-canvas` intentionally accepts only closed polygonal contours. Passing
curves into the canvas would widen its rasterizer substantially; flattening
them in the font parser would attach a pixel-quality choice to a portable,
exact geometry boundary.

A naive recursive converter can also allocate once per curve or let a very
large transformed glyph produce excessive work and memory during a frame.

## Decision

Add `anodrel-glyph`, a separate first-party portable adapter that depends only
on `anodrel-font` and `anodrel-canvas`. It converts an already-validated
`GlyphPath` into a canvas `Path` through a validated baseline and scale.
The transform converts upward TrueType coordinates to downward canvas pixels.

The adapter flattens a quadratic only after that device transform. It uses
iterative de Casteljau subdivision and accepts a chord when its control point
is within one quarter of a canvas pixel. It has a fixed stack, a maximum depth
of eight, and a 65,536-vertex whole-glyph limit. A limit that cannot maintain
the error target is a closed error, never a lower-quality result or partial
path. Completed contour vectors move into the canvas path without another
copy.

## Consequences

- Exact font geometry stays portable and does not depend on the rasterizer.
- The quality target is explicit in physical pixel space and scales with the
  displayed glyph rather than with arbitrary source-font units.
- The software canvas continues to rasterize one primitive: closed polygons.
- Glyph preparation has a bounded work and memory envelope appropriate for a
  future retained coverage cache.

## Deliberately absent

- font parsing, source selection, composite glyphs, metrics, shaping, fallback,
  text layout, hinting, kerning, and a public application text API;
- coverage-mask creation, cache policy, painting, GPU work, host integration,
  or a Linux application window;
- caller-defined tolerance, hidden global state, platform APIs, and third-party
  font or graphics code.

## Alternatives considered

**Flatten inside `anodrel-font`.** This would mix exact source geometry with
one destination renderer's coordinate system and quality policy. Refused.

**Teach `anodrel-canvas` quadratic curves.** Every raster and path operation
would then need another primitive, although glyph conversion is the only
current consumer. Refused.

**Use unrestricted recursive subdivision.** This risks unbounded stack, heap,
and frame work from one glyph. Refused.

## Revisit conditions

Revisit before configurable quality, composite glyphs, glyph metrics, retained
coverage caching, direct scan conversion, a text layout API, or an application
font capability.
