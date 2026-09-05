# Anodrel Renderer

**Status:** Portable and used by every first-party Windows surface.

This is the public contract for how Anodrel draws. It covers two portable
crates — `anodrel-canvas` (how to draw) and `anodrel-brand` (what Anodrel looks
like) — and the seam a native host implements to put their output on screen.
`anodrel-glyph` is a separate, narrow adapter that supplies flattened glyph
contours to the canvas, and `anodrel-text` supplies bounded unshaped glyph runs;
see `docs/GLYPH_RENDERING.md` and `docs/TEXT_RUNS.md`.

Neither crate depends on an operating system or on a third-party library, and
both are `#![forbid(unsafe_code)]`. The reasoning behind that is Decision 0013.

## Contents

- [Model](#model)
- [Coordinates](#coordinates)
- [`anodrel-canvas`](#anodrel-canvas)
  - [Color](#color)
  - [Point and Rect](#point-and-rect)
  - [Path](#path)
  - [Paint](#paint)
  - [Canvas](#canvas)
  - [Mask](#mask)
  - [Image](#image)
  - [Bevel](#bevel)
- [`anodrel-brand`](#anodrel-brand)
  - [palette](#palette)
  - [mark](#mark)
  - [icon](#icon)
- [Host integration](#host-integration)
- [Performance](#performance)
- [Testing](#testing)
- [Limits](#limits)

## Model

Five types carry the whole design.

| Type | Role |
| --- | --- |
| `Color` | A straight-alpha RGBA value. |
| `Path` | One or more closed polygonal contours. |
| `Paint` | A pure function from a point to a `Color`. |
| `Canvas` | A 32-bit pixel buffer that paths are composited into. |
| `Mask` | A standalone coverage buffer that can be blurred. |
| `Image` | A raster that can be filtered, scaled, and composited. |

Drawing is always the same shape: build a `Path`, choose a `Paint`, composite it
into a `Canvas`. There is no retained scene, no state stack, and no implicit
current transform — a call does exactly what its arguments say.

Curves are flattened by the path builders, so the rasterizer has one primitive
to handle. Contours combine under the **non-zero winding rule**: a contour wound
against the one enclosing it becomes a hole.

## Coordinates

Canvas space is in pixels with `y` increasing downward. Pixel centres sit at
half-integer coordinates, so a rectangle from `2.0` to `6.0` covers exactly the
four pixels `2..=5`. Everything outside the canvas is clipped; drawing far
off-surface is a no-op, never an error.

## `anodrel-canvas`

```rust
use anodrel_canvas::{Bevel, Canvas, Color, Mask, Paint, Path, Point, Rect, Stop, point, stop};
```

### Color

Straight (non-premultiplied) alpha, in RGBA order. Premultiplication happens
only at the moment of compositing.

| Method | Meaning |
| --- | --- |
| `Color::rgb(r, g, b)` | Opaque colour. |
| `Color::rgba(r, g, b, a)` | Colour with explicit opacity. |
| `Color::hex(0xRRGGBB)` | Opaque colour from a design literal. |
| `with_alpha(a)` | Same colour at a new opacity. |
| `scale_alpha(factor)` | Opacity scaled by `0.0..=1.0`. How reveals fade tokens. |
| `lerp(other, t)` | Interpolates every channel including alpha. |
| `lighten(t)` / `darken(t)` | Mixes toward white or black, preserving alpha. |
| `over(backdrop)` | Source-over against an opaque backdrop. Always opaque. |
| `to_argb()` / `from_argb(v)` | Packs as `0xAARRGGBB`. |

`to_argb` is the byte order a 32-bit `BI_RGB` bitmap wants on a little-endian
target, which is why presenting a canvas needs no conversion pass.

### Point and Rect

`Point` doubles as a vector: `offset`, `to`, `length`, `normalized`, `dot`,
`scale`, `lerp`. `normalized` returns `None` for a degenerate vector rather than
producing a NaN.

`Rect` is `left`, `top`, `right`, `bottom`. It is empty when `right <= left` or
`bottom <= top`, and empty rectangles are treated as no-ops throughout. Useful
constructors and operations: `from_size`, `centered`, `inflate`, `translate`,
`intersect`, `union`, `contains`, `center`, `width`, `height`.

### Path

| Constructor | Shape |
| --- | --- |
| `Path::polygon(points)` | One closed contour; do not repeat the first point. |
| `Path::rect(rect)` | Rectangle. |
| `Path::rounded_rect(rect, radius)` | Radius clamped to half the shorter side. |
| `Path::circle(center, radius)` | Circle. |
| `Path::ellipse(rect)` | Ellipse inscribed in a rectangle. |
| `Path::ring(center, outer, thickness)` | Annulus; the inner contour is reversed. |
| `push_owned_contour(points)` | Transfers one completed contour without copying it. |

Transforms return new paths: `translate`, `scale_about`, and `fit_unit_square`,
which maps geometry authored in a normalised `0.0..=1.0` square into a target
rectangle. Brand geometry is authored that way and placed with `fit_unit_square`.

Two methods support bevelling:

- **`inset(distance)`** returns the path moved `distance` toward its interior.
  Adjacent offset edges are intersected, so corners keep a mitre instead of
  splaying open; nearly parallel edges fall back to a plain normal offset, and a
  very sharp corner is clamped rather than producing a spike. The result is
  independent of the contour's winding direction.
- **`bevel_bands(distance)`** returns the quads tiling the band between a path
  and its inset, each paired with the outward unit normal of the edge it came
  from. Bands meet exactly at the mitred corners, so translucent overlays never
  double up.

`Mask::for_path_bounded` is the checked counterpart to `Mask::for_path` for a
caller that owns an explicit pixel budget. It returns `None` before allocation
when the path or padding is non-finite, its bounds cannot be represented, or
the requested area is above that budget. Glyphs use it through `anodrel-glyph`;
authored renderer surfaces continue to use the unconstrained constructor.

### Paint

A paint is a pure function of position, so the rasterizer samples it at pixel
centres with no state carried between spans.

```rust
Paint::solid(color)
Paint::linear(start, end, stops)          // stops: Vec<Stop>
Paint::vertical(top, bottom, from, to)    // two-colour shorthand
Paint::horizontal(left, right, from, to)  // two-colour shorthand
Paint::radial(center, radius, stops)
```

A `Stop` is a position in `0.0..=1.0` and a colour. Positions outside the first
and last stop clamp to the end colours; gradients do not repeat. A degenerate
axis or a zero radius is handled rather than dividing by zero, and an empty stop
table samples transparent.

`scale_alpha(factor)` returns a copy with every stop faded, which is how an
animated element dims without its stop table being rebuilt at the call site.

### Canvas

```rust
let mut canvas = Canvas::new(width, height);   // starts fully transparent
canvas.clear(Color::hex(0x05070F));            // opaque fill
```

A new canvas is transparent so it can serve as a source with real alpha — the
window icon relies on this. Surfaces that own their whole area call `clear`
first.

| Method | Effect |
| --- | --- |
| `fill_path(path, paint)` | Composites a shape with antialiased edges. |
| `fill_rect` / `fill_rounded_rect` / `fill_circle` | Shorthands. |
| `stroke_path(path, width, paint)` | Outline centred on the path's edges. |
| `stroke_rounded_rect(rect, radius, width, paint)` | Shorthand. |
| `draw_line(from, to, width, paint)` | Single segment. |
| `draw_polyline(points, width, paint)` | Connected run with rounded joins. |
| `fill_mask(mask, paint)` | Composites externally produced coverage at its own origin. |
| `fill_mask_offset(mask, x, y, paint)` | Composites retained coverage at one extra whole-pixel offset without copying it. |
| `fill_beveled(path, face, bevel)` | Fill plus directional edge shading. |
| `draw_glow(path, radius, passes, paint)` | Soft bloom around a shape. |
| `draw_shadow(path, offset, radius, color)` | Blurred offset shadow. |
| `copy_from(other)` | Copies a same-sized canvas; `false` if sizes differ. |
| `draw_canvas_clipped(other, x, y, opacity, clip)` | Composites a layer inside one explicit destination rectangle. |
| `pixel(x, y)` / `sample(point)` | Reads back a pixel. |
| `pixels()` | Packed `0xAARRGGBB`, row-major — the buffer to present. |

Compositing is source-over. Against an opaque destination it reduces to a plain
interpolation, which is the path an Anodrel surface takes almost everywhere; the
general form is used when the destination is itself translucent.

`pixel` matters more than it looks: it lets a host resolve a colour against the
exact pixels already rendered underneath, which is what a platform text API with
no alpha needs.

### Mask

A standalone coverage buffer positioned by an origin in canvas space and sized
to the geometry it holds, so a glow does not pay for the whole surface.

```rust
let mut mask = Mask::for_path(&path, padding);
mask.blur(radius);
canvas.fill_mask(&mask, &paint);
```

- `Mask::for_path(path, padding)` — sized to the path's bounds plus padding.
- `Mask::from_coverage(x, y, w, h, coverage)` — wraps externally produced
  coverage, returning `None` if the length does not match. **This is the seam a
  host uses to bring platform-rendered glyphs into the canvas.**
- `Mask::positioned(x, y)` — an owned copy at a new origin, for a caller that
  needs an independently mutable mask.
- `Canvas::fill_mask_offset(mask, x, y, paint)` — reads a retained mask at an
  extra whole-pixel offset without changing or copying its coverage; this is
  the composition half of the bounded glyph cache.
- `blur(radius)` — three box passes approximating a Gaussian. Cost is linear in
  the masked area and independent of radius.

### Image

A straight-alpha raster that participates in the same pipeline as everything
else — scaled, faded, and composited with the same blend as a vector fill.

```rust
let asset = Image::from_bgra_bytes(512, 512, bytes)?;   // pre-decoded pixels
let reduced = asset.resized(256, 256);                  // filtered once
canvas.draw_image(&reduced, dest, opacity);             // bilinear placement
```

| Method | Effect |
| --- | --- |
| `from_bgra_bytes(w, h, bytes)` | Wraps packed `B, G, R, A`; `None` on a size mismatch. |
| `from_pixels(w, h, pixels)` | Wraps `Color` values in row-major order. |
| `resized(w, h)` | Filtered resample; each destination pixel averages its full source footprint. |
| `sample_bilinear(u, v)` | Normalised sample, clamped at the edges. |
| `cropped(x, y, w, h)` | Sub-rectangle. |
| `opaque_bounds(threshold)` | Tight bounds of the artwork inside its padding. |
| `to_bgra_bytes()` | Packed bytes back out. |
| `Canvas::draw_image(image, dest, opacity)` | Composites into a rectangle. |

**Filtering is done in premultiplied space throughout.** Averaging straight
alpha lets a transparent pixel's colour bleed into its neighbours, which shows
up as a dark fringe around every edge of a cut-out. This is the single most
common way raster logo handling goes wrong, and it is asserted against.

**Reduce once, then draw.** `draw_image` samples bilinearly, which cannot
represent a footprint several pixels wide. For a large reduction call `resized`
once and draw from the result; re-filtering every frame would be wasteful
anyway. `anodrel-brand` does exactly this, caching reductions in size buckets so
an animated mark resamples a handful of times rather than once per frame.

### Bevel

Directional edge shading — the effect that makes the mark read as solid.

```rust
canvas.fill_beveled(&path, &face_paint, &Bevel::top_left(depth));
```

The model is deliberately flat: every edge is a chamfer of constant `depth`, and
its shading is the dot product of the edge's outward normal with the light
direction. Edges facing the light take a white overlay scaled by `highlight`;
edges facing away take a black overlay scaled by `shadow`. One dot product per
edge, and it is enough to make a two-dimensional polygon read as an extruded
piece.

`Bevel::top_left(depth)` is the brand's lighting. `with_strength` and `scaled`
adjust it; `scaled(0.0)` removes every overlay, which is how a mark fades in.

## `anodrel-brand`

```rust
use anodrel_brand::{Icon, mark, mark::MarkStyle, palette};
```

The identity ships as the authored artwork, committed pre-decoded, plus
geometry for the sizes a raster cannot serve. Decision 0015 records the split
and corrects an earlier decision that had geometry as the source of truth.

### palette

Every colour a first-party surface may use, named. Surfaces reference tokens
rather than literals so the identity can be adjusted in one place and a review
can tell brand colour from arbitrary colour.

- **Mark ramp** — `VIOLET_LIGHT`, `VIOLET`, `VIOLET_DEEP`, `INDIGO`, `BLUE`,
  `BLUE_LIGHT`, `SKY`.
- **Surfaces** — `BACKDROP`, `BACKDROP_LIFT`, `CHROME`, `PANEL`,
  `PANEL_RAISED`, `PANEL_EDGE`.
- **Ink** — `INK`, `INK_SOFT`, `INK_MUTED`.
- **Signal** — `READY`, `PLANNED`.
- **Accents** — `ACCENT_CORE`, `ACCENT_PACKAGE`, `ACCENT_IPC`, `ACCENT_SHELL`.

`palette::mark_ramp()` returns the left-to-right violet-to-blue ramp as
positions paired with colours, so a gradient across the mark and a matching ramp
across a row of cards come from one source.

### mark

```rust
mark::draw(&mut canvas, bounds, MarkStyle::hero());
```

One call, two paths behind it:

- **At or above `RASTER_MIN_EDGE` (64 px)** — the authored artwork, reduced to
  a cached size bucket and composited. The glow is taken from the artwork's own
  alpha channel, so the light matches the mark being lit rather than an
  approximation of it.
- **Below 64 px** — the geometry, because a raster reduced that far loses the
  chamfers that make the mark read as solid, while a vector is rasterized at the
  size actually asked for.

Both occupy identical bounds — the asset is cropped square about its artwork and
the geometry fills the unit square — so nothing shifts at the threshold. A test
asserts the painted extent agrees across it.

The geometry is the `A` cut into four pieces: `Apex`, `LeftLeg`, `RightLeg`,
`Crossbar`. The gaps between them are part of the identity, not spacing to be
tuned.

| Item | Purpose |
| --- | --- |
| `mark::raster()` | The authored asset at `RASTER_SIDE`; `None` if it disagrees with that constant. |
| `mark::draw(canvas, bounds, style)` | The whole mark, choosing a path by size. |
| `Piece::ALL` | Every geometry piece, in back-to-front painting order. |
| `Piece::unit_path()` | One piece's outline in the unit square. |
| `mark::pieces(bounds)` | Every piece fitted to a rectangle. |
| `mark::silhouette(bounds)` | All four as one path — the glow shape for the geometry path. |
| `mark::face_paint(bounds)` | The horizontal violet-to-blue ramp. |
| `mark::depth_paint(bounds)` | The vertical shading laid over each piece. |

`MarkStyle::hero()` is the full treatment; `MarkStyle::compact()` drops the glow
and widens the chamfer proportionally, because at small sizes a hairline bevel
disappears into a single pixel of coverage. `with_opacity` fades the whole mark.

Draw order inside `draw` is deliberate: the glow is laid down first so the
faceted pieces sit on top of their own light, then each piece is filled with the
shared ramp, shaded vertically, and chamfered — in that order, so the chamfers
read as edges of a solid rather than as outlines.

### icon

Line glyphs: `Core`, `Package`, `Ipc`, `Shell`, `Launch`, `Logs`, `Inspect`,
`Diagnostics`. Each is open polylines in a unit square, stroked at draw time.
Stroking rather than filling keeps one weight across the set, so glyphs stay a
family at any size.

```rust
Icon::Core.draw(&mut canvas, bounds, stroke_width, &paint);
```

A non-square target pads to the inscribed square rather than distorting the
artwork.

## Host integration

A host supplies three things. The Windows host is the reference implementation.

**1. Presenting a canvas.** Compose the client area into a canvas and blit it
once. `canvas.pixels()` is already in the right byte order for a 32-bit
`BI_RGB` top-down bitmap; a negative bitmap height selects top-down row order so
nothing needs flipping. One call per frame means a partial frame is never
visible, which is why the window class carries no background brush and
`WM_ERASEBKGND` is answered directly.

**2. Text.** The current Windows host owns its temporary GDI font, shaping, and
hinting route. It draws glyphs into a private memory bitmap, lifts the grey
levels out as coverage, and wraps them with `Mask::from_coverage`. Everything
after that is canvas compositing — which is what buys gradient-filled type, real
opacity during a reveal, and type that is part of the same single blit as the
graphics. The first-party font, glyph, and text-run foundations are not yet
connected to this painter, so they are not presented as a replacement for it.

**3. Density.** Opt into per-monitor DPI awareness before creating a window. A
renderer that produces its own antialiasing must not then be scaled by the
system. Author layout against a base size and derive a scale from the client
area; the same code then serves every density without a separate path.

## Performance

Filling costs time proportional to the area a shape actually covers, not to the
size of the canvas, so a surface made of many small pieces stays cheap. Blur is
linear in the masked area and independent of radius. Nothing is cached inside
the canvas; a surface is expected to be composed once per paint.

Caching is therefore the caller's job, and four caches carry the animated
surface. Three belong to the host:

- **Glyph runs**, keyed by their text and typographic settings. A reveal
  repaints the same strings many times per second while only their colour and
  position change, so rasterizing each run once is the difference between a
  smooth reveal and a stuttering one.
- **The backdrop**, keyed by client size. It is a full-surface radial gradient —
  around a million paint samples — and is identical on every frame, so computing
  it once per size turns the per-frame cost into a copy.
- **The settled base**, keyed by client size and the host-visible Startup Lab
  state. It contains everything below or outside the animated mark. An ambient
  update restores only the mark and foreground-detail band from that base, then
  repaints the mark, title, identity, and validation pill in their original
  order. The foreground band is intentionally wider than the mark because
  translucent type must be restored before it is repainted. A host test asserts
  that this partial path is pixel-identical to a full settled compose.

The fourth belongs to the brand crate:

- **The mark's blurred glow**, keyed by the source raster bucket, the mark's
  size in whole pixels, the padding, and the box radius the blur will actually
  apply. A retained mask is repositioned to whole pixels and composited without
  being rebuilt or copied.

That last one is the only cache here that is not exact, and the difference is
deliberate: see [Decision 0064](decisions/0064-retained-raster-effects-trade-bounded-fidelity.md).
A reveal moves the mark by a fraction of a pixel per frame while the blur
spreads its alpha over tens of pixels, so rebuilding the mask for that
difference cost about 2 ms a frame and changed at most 7 levels out of 255 at
the half-pixel worst case — nothing at all when the mark lands on the pixel
grid. `a_retained_glow_matches_one_built_in_place` composites both paths over
the backdrop they are drawn on and holds them to that bound.

Two canvas methods exist for this and are worth knowing about before writing
another retained effect:

| Method | Use |
| --- | --- |
| `Mask::blur_box_radius(radius)` | The radius `blur` will actually apply. Key a retained mask on this, not on the radius asked for. |
| `Mask::reposition(x, y)` | Moves a mask in place. A mark-sized glow is around half a megabyte of coverage that `positioned` would copy. |

Measured on the Startup Lab at 1240×900 in a release build, a reveal frame
composes in roughly **6.7 ms** and its most expensive sustained frame in
roughly **8.0 ms**, against the animation timer's 16 ms interval. The
`frame_budget` guards assert both, so the budget is enforced rather than
noticed by eye; `docs/PERFORMANCE.md` records how they are measured and why
their statistic is a minimum rather than a single run.

An unoptimised build is about ten times slower and cannot hold the frame rate.
This is why `start.bat` builds in release; it is a requirement, not a
preference.

The largest remaining cost is sampling a gradient per pixel under a large
blurred mask: compositing the mark's glow twice through its gradient is about
2.3 ms of a reveal frame, now that building the mask no longer is.

The perf lab's `--renderer` workload puts a number on why. Per pixel, a flat
fill costs about **0.07 ns** and the same pixel filled through a gradient costs
**19 ns** — two to three orders of magnitude more, purely for evaluating the
paint. See `docs/PERFORMANCE.md` for the full stage table and what the numbers
exclude.

### What has already been tried

**Hoisting the gradient's invariants out of the fill loop.** `Paint::sample`
recomputes a linear gradient's axis and its squared length for every pixel,
which looks like obvious waste. Preparing those once per fill and sampling
through a borrowed struct — bit-identical output, held to that by a test —
produced **no measurable improvement**: measured A/B against an unchanged
binary, the two stages that use a paint moved less than stages that cannot have
changed. LLVM already hoists that arithmetic, because the paint is borrowed
immutably for the whole fill. The change was reverted rather than kept, since
unpaid complexity is still complexity.

The remaining cost was the per-pixel stop lookup and four-channel `lerp` under
the diffuse mark glow. Decision 0176 now replaces only that effect with an
explicit 512-sample colour ramp. Its brand test compares every sampled glow
colour with the exact gradient and permits at most one level in any RGBA
channel; exact paints remain the default elsewhere. The renderer workload
reports both mask-fill paths: its optimized same-process 11-sample run measured
1.779 ms for exact fill and 1.113 ms for the ramp. The Windows release guard
remains authoritative for a whole sustained frame.

### Measuring a renderer change honestly

The attempt above is also the method. A single before-and-after pair proved
nothing here: the first baseline was taken on a cold machine and its *control*
stage — one that touches no paint — moved 28% on its own. What worked was
building both binaries, alternating runs, and reading the controls first. If a
stage that cannot have changed moves as much as the stage under test, the
measurement is of the machine.

## Testing

Because neither crate touches an operating system, rendering is tested by
asserting on pixels, headless and deterministically:

- **Coverage** — a pixel-aligned rectangle fills exactly its pixels; a half
  covered pixel receives half coverage.
- **Fill rule** — a ring leaves its centre untouched.
- **Compositing** — opacity composites against the backdrop; a transparent
  canvas accumulates alpha rather than dragging colour toward black.
- **Clipping** — drawing far outside the canvas is clipped, not a panic.
- **Blur** — coverage spreads past the original edge and conserves its energy to
  within 5%.
- **Bevels** — a lit square is brighter on top than on the bottom.
- **Geometry** — the mark is symmetric about its centre line, its legs mirror
  each other, its pieces share a baseline and are separated by visible gaps, and
  every glyph stays inside its unit square.
- **The authored asset** — it loads at its declared size, is a clean cut-out
  (transparent corners, empty negative space, so the draw-time glow cannot
  double up), its artwork reaches every edge of its crop, and its legs carry the
  brand gradient.
- **Raster/geometry agreement** — the painted extent matches on both sides of
  the size threshold, so the mark cannot shift when the path changes.
- **Premultiplied filtering** — reducing a checkerboard lands on the mean rather
  than aliasing, and a colour beside a transparent pixel keeps its hue instead
  of darkening.

The host adds tests for the parts only it can cover: full and partial blits are
verified pixel-for-pixel into a memory bitmap including row order and source
alignment, glyph runs measure and align correctly, and every surface composes
at each supported client size.

## Limits

Deliberately absent, and the reason:

- **No transform stack.** Paths carry their own placement. A global transform
  would make a call's effect depend on invisible state.
- **No retained clip regions.** Drawing is clipped to the canvas bounds. A
  pre-rendered layer may be composed through one explicit rectangle with
  `draw_canvas_clipped`, but that clip cannot affect later calls.
- **No curves in the pipeline.** Builders flatten; the rasterizer handles
  polygons only.
- **No text layout.** The canvas composites glyph coverage. Line breaking and
  measurement belong to the host, which owns the font stack.
- **No image decoding.** Nothing parses PNG, JPEG, or any other container. The
  one shipped raster is committed pre-decoded, so loading is a length check.
- **No incremental repaint.** A surface is composed whole each frame.

Each is a gate a future need would have to justify opening, not an oversight.
