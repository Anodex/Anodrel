# Decision 0212: Windows owned-text probe stays fixed and internal

- Status: Accepted
- Date: 2026-09-05

## Context

The Windows host can now obtain one bounded private copy of its fixed GDI
surface face, and Anodrel already owns the parser, unshaped run builder,
glyph-coverage cache, and canvas compositor. Those pieces had not exercised one
live Windows face together. Replacing the existing GDI text route at this point
would remove shaping and hinting from visible surfaces before the owned route
has a quality or performance comparison.

## Decision

Add one host-private, non-windowed probe that accepts no application input. It
uses the fixed ASCII value `ANODREL` at one fixed 32-pixel em size, renders it
twice at whole-pixel-separated baselines, and performs this exact chain:

~~~text
selected GDI face bytes
    -> FontFace
    -> TextRun
    -> face-local GlyphMaskCache
    -> Canvas::fill_mask_offset
~~~

The two rows deliberately share glyph, scale, and fractional baseline phases,
so the second row proves cache reuse while each returned whole-pixel offset
still controls its own composition. The probe returns only private result facts
for its host test: source-order glyph count, retained cache count, and the
finished canvas. It has no command-line mode, host API, protocol operation,
application callback, native window, file output, telemetry, or fallback to the
GDI painter.

Every existing limit remains active: the 8 MiB face source cap; the text run's
1,024-scalar and signed-advance limits; the glyph path and per-mask 262,144
pixel limits; and the glyph cache's 64-mask, 2,097,152-pixel budget. Any closed
failure stops the entire probe with no partial success claim.

The current GDI painter remains the sole visible text route. This probe is
evidence that the owned pieces compose correctly on the selected Windows face,
not a quality, shaping, accessibility, startup, or frame-performance claim.

## Consequences

- Windows obtains a small, repeatable integration check across the owned text
  layers without extending application authority or changing a pixel on a
  released host surface.
- The check exercises true selected-face data rather than only synthetic font
  fixtures, and verifies that cache reuse does not require copying coverage.
- A later painter can build on an explicit known-good chain, but must still
  establish source retention, real layout, clipping, invalidation, parity,
  accessibility agreement, and performance before presentation changes.

## Deliberately absent

- a general text draw API, arbitrary text, configurable font size, fallback,
  shaping, wrapping, hinting, editing, readback, or application font control;
- a retained live face, a process-wide cache, a performance metric, a screenshot,
  a window, a protocol field, or a command-line diagnostic; and
- any change to the GDI painter, UI document, accessibility provider, Linux,
  macOS, or product release status.

## Alternatives considered

**Replace GDI with the owned path now.** The owned path is intentionally
unshaped and unhinted. This would make visible text worse before a measured
comparison exists. Refused.

**Exercise the layers only with synthetic faces.** Useful unit evidence already
exists, but cannot prove that the selected Windows source completes the same
chain. Insufficient.

**Expose a generic development drawing command.** It would create a new input,
rendering, and observability boundary solely to test a fixed internal fact.
Refused.

## Revisit conditions

Revisit before allowing arbitrary text, retaining a parsed face or cache,
painting an application surface, adding quality/performance comparison,
supporting fallback or shaping, or changing the accessibility tree.
