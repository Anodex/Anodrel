# Anodrel Glyph Rendering

**Status:** Portable quadratic-to-canvas path foundation.

`anodrel-glyph` is the narrow first-party adapter between validated
`anodrel-font` geometry and the closed polygon contours accepted by
`anodrel-canvas`. It prepares one simple glyph for a later host-owned coverage
mask. It does not load a font, choose a character, calculate metrics, lay out
text, or paint pixels itself.

## Boundary

The crate depends only on the two owned portable crates `anodrel-font` and
`anodrel-canvas`. It has no operating-system calls, file access, global cache,
third-party dependency, or unsafe code. `anodrel-font` remains responsible for
validating source bytes and preserving exact design geometry; `anodrel-canvas`
remains responsible for non-zero winding and coverage rasterization.

The public conversion is deliberately one way:

```rust
let placement = anodrel_glyph::GlyphPlacement::new(baseline, pixels_per_design_unit)?;
let canvas_path = anodrel_glyph::canvas_path(&glyph_path, placement)?;
```

`canvas_path` consumes an already-validated `GlyphPath` by reference and
returns a normal `anodrel_canvas::Path`. It exposes no editable font geometry,
pixel readback, cache handle, source identity, or platform state.

## Coordinates

`GlyphPlacement` makes the one required coordinate-system change explicit.
TrueType design coordinates rise upward from a baseline, while canvas pixels
rise downward. For a source point held in doubled design units, the output is:

~~~text
x_pixel = baseline.x + (x_twice / 2) × pixels_per_design_unit
y_pixel = baseline.y - (y_twice / 2) × pixels_per_design_unit
~~~

The baseline coordinates must be finite and lie within ±1,048,576 pixels.
`pixels_per_design_unit` must be finite, greater than zero, and no greater than
64. These are renderer-input bounds, not a font-size API: metrics and user
text sizing remain later decisions.

## Curve conversion

Lines pass through directly. Quadratic Bézier curves use iterative de Casteljau
subdivision until the control point is at most one quarter of a pixel from its
chord. The test is performed in canvas pixel space, so the same glyph gains
detail only when it is actually displayed larger.

The adapter uses a fixed local subdivision stack rather than allocating per
curve. A curve may subdivide at most eight times, creating at most 256 line
segments. A whole glyph may produce at most 65,536 vertices. If either bound
would prevent the stated one-quarter-pixel target, conversion returns a closed
`GlyphRenderError::TooComplex` result and returns no partial path.

Each completed contour transfers its owned vertex vector directly to the canvas
path. The repeated closing vertex is removed because `anodrel-canvas::Path`
closes contours implicitly. Degenerate contours remain the canvas layer's
existing no-op rather than becoming special text behavior.

## Deliberately absent

- font discovery, file formats, character maps, composite glyphs, metrics,
  hinting, kerning, shaping, fallback, line breaking, and text layout;
- per-application font configuration, a protocol operation, cached glyph runs,
  GPU work, raster-mask allocation, drawing, or host presentation;
- a Linux application surface, desktop input route, or accessibility tree.

The next rendering step may rasterize this returned `Path` into a retained
coverage mask only after it defines cache keys, bounds, clipping, and
invalidation. It must not move source-font or layout authority into this
adapter.

## Verification

The adapter has deterministic unit tests for placement direction and limits,
flat lines, curved subdivision, exact closure removal, and complexity refusal.
It is compiled and linted on Windows and Linux alongside the native workspace.
