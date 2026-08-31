# Decision 0135: TrueType contours convert to exact quadratic paths

- Status: Accepted
- Date: 2026-08-30

## Context

Decision 0134 returns a validated TrueType simple outline, but its on- and
off-curve point sequence is not directly drawable. A renderer needs explicit
line and quadratic segments, including implied on-curve points between adjacent
off-curve controls and at an off-curve contour boundary.

Converting coordinates to floating point here would make half-unit midpoint
behavior architecture- and later-scaling-dependent. Building one heap vector
per contour would also turn a complex but valid glyph into a burst of small
allocations on a rendering-adjacent path.

## Decision

Add a pure `GlyphOutline::quadratic_path` conversion in `anodrel-font`. It
returns closed contour starts plus one flat line/quadratic segment buffer and
contour-end indexes. Coordinates use doubled signed design units: source
coordinates are doubled and implied midpoints are summed, so every midpoint is
exact without a floating-point representation.

The converter consumes only the already-owned, validated `GlyphOutline`. It
does not parse bytes, recover malformed contours, choose a fill rule, flatten
a curve, inspect a font, call an operating-system API, cache globally, or add a
protocol operation. Its work is linear in the bounded outline point count and
uses a constant number of output allocations.

## Consequences

- A later software rasterizer receives canonical quadratic geometry without
  duplicating TrueType's implied-point rules in each host.
- Exact doubled units keep geometry deterministic until a dedicated scaling and
  scan-conversion policy is chosen.
- The font parser remains a small first-party geometry source, not a renderer
  or a general-purpose vector API.
- Composite glyphs remain unsupported; no conversion claim implies their
  transforms or recursive components have been handled.

## Deliberately absent

- composite outlines, transforms, point attachment, recursion, metrics,
  hinting, shaping, fallback, kerning, and line layout;
- curve flattening, winding evaluation, scan conversion, antialiasing,
  raster masks, canvas calls, GPU work, or a Linux text surface;
- float coordinates, hidden font loading, caching, or application control of
  any font data.

## Alternatives considered

**Flatten curves while parsing.** This bakes a pixel-quality choice into font
geometry and costs work even when the caller only wants to retain a path.
Refused.

**Use floating-point midpoint coordinates.** Convenient, but it needlessly
loses the exact half-unit relationship that TrueType contours require. Refused.

**Have every renderer reinterpret off-curve points.** This duplicates a subtle
closed-contour rule across hosts and invites different outputs. Refused.

## Revisit conditions

Revisit before composite glyph support, a scaling format, curve flattening,
rasterization, a glyph cache, a font source, or a Linux application text path.
