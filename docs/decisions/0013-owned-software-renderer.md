# Decision 0013: Draw first-party surfaces with an owned software renderer

**Status:** Accepted. The asset reasoning below is superseded by Decision 0015.

**Date:** 2026-07-31

> **Correction.** Option 2 below claims an image asset needs a decoder and
> cannot be tested. Both are wrong: a pre-decoded asset needs no decoder, and it
> can be asserted on. The renderer decision stands; the mark is now the authored
> artwork. See Decision 0015.

## Context

The Startup Lab drew directly with GDI primitives: `FillRect`, `Ellipse`,
`MoveToEx`/`LineTo`, and `DrawTextW`. That set cannot produce the platform's
identity. GDI has no antialiasing for filled geometry, no gradient fill, no
alpha compositing, and no blur, so the Anodrel mark could only be approximated
with circles and lines. Drawing straight to the window device context also
meant partial frames were visible during a resize.

Three ways forward were considered:

1. **Adopt a graphics library** (Direct2D, Skia, tiny-skia). Direct2D is an
   operating-system API, but it is a large COM surface that would have to be
   re-abstracted for every future host; the others are third-party runtime
   dependencies, which Decision 0005 rules out.
2. **Ship the identity as an image asset.** ~~This needs a decoder the project
   does not own, fixes the artwork at one resolution, and cannot be asserted on
   in a test.~~ Only the resolution limit was a real objection; see the
   correction above and Decision 0015.
3. **Own a small rasterizer.** A coverage-based scanline filler with gradients
   and a blur is a few hundred lines and has no dependency at all.

## Decision

First-party surfaces are composed by two owned, portable crates and presented
by the host in a single blit.

`anodrel-canvas` is a software rasterizer: closed polygonal paths, non-zero
winding, coverage antialiasing, linear and radial gradients, source-over
compositing, blurred coverage masks, and directional edge bevels. It has no
operating-system dependency, no third-party crate, and `#![forbid(unsafe_code)]`.

`anodrel-brand` holds the visual identity as data: the colour tokens, the
four-piece `A` mark as normalised geometry, and the icon set. It draws through
`anodrel-canvas` and is equally portable.

The Windows host keeps the platform-specific parts and nothing more:

- the client area is composed into an owned canvas and reaches the screen
  through one `StretchDIBits` call, so a frame is never partially visible;
- text is rasterized by GDI into a private memory bitmap and lifted out as a
  coverage mask, so Windows still owns fonts and shaping while compositing,
  opacity, and gradient fills stay with the canvas;
- the window icon is generated at run time from the same mark, so no icon
  resource is compiled into the executable;
- the process opts into per-monitor DPI awareness, because a renderer that
  produces its own antialiasing must not then be scaled by the system.

Surfaces are authored against a base size and scaled, so one layout serves every
display density.

## Consequences

Positive:

- the identity is drawn correctly at any size, with the authored artwork above
  64 px and unit-tested geometry below it (Decision 0015);
- rendering is portable: a future macOS or Linux host reuses both crates and
  supplies only a blit and a glyph source;
- drawing is testable without a window — the pixel assertions in
  `anodrel-canvas` and the surface tests in the host run headless;
- flicker is gone by construction rather than by suppression.

Tradeoffs:

- the project now owns rasterization correctness, including fill rules,
  antialiasing quality, and compositing;
- software rendering costs real time. A frame of the Startup Lab is measured and
  asserted to fit inside the animation timer's interval, and `start.bat` builds
  in release because an unoptimised build cannot hold that rate;
- text is grayscale antialiased rather than subpixel antialiased, which is the
  cost of compositing type through the canvas instead of drawing it last;
- expensive invariant layers are cached in the host (the backdrop, glyph runs),
  so cache invalidation is now a host concern.

## Revisit conditions

Reconsider if a surface needs typographic control beyond single-line runs
(bidirectional text, complex shaping, justified paragraphs), if animation grows
beyond a bounded startup reveal into continuous motion, or if a host target
offers a first-party accelerated API that meets the ownership rule and covers
every platform in the plan.
