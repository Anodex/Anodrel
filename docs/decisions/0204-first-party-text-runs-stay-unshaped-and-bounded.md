# Decision 0204: First-party text runs stay unshaped and bounded

- Status: Accepted
- Date: 2026-09-05

## Context

Anodrel can parse one owned TrueType face, read its horizontal metrics, extract
one simple outline, and prepare a bounded coverage mask. It cannot yet connect
two glyphs: there is no first-party value that preserves glyph source order and
horizontal pen positions before a host selects device coordinates.

Leaving that gap to each host would make an owned glyph path depend on hidden
platform text layout. Bringing in a text stack would add a shipped third-party
runtime. Treating an unbounded string as a run would also make one value choose
unbounded allocation and later raster work.

## Decision

Add portable `anodrel-text`. Given one validated `FontFace` and UTF-8 text, it
returns at most 1,024 scalar-mapped glyphs in source order with unscaled
horizontal pen positions and advances. It requires complete validated
horizontal metrics and refuses the complete request when any scalar lacks a
nonzero glyph, a metric is invalid, the glyph count is too large, or total
advance would exceed 1,048,576 design units.

The crate depends only on the owned font parser, forbids unsafe code, and has
no filesystem, native, cache, protocol, or global state. It returns glyph IDs
and design-unit facts, not text, source bytes, device positions, or a draw
command.

## Consequences

- A future Windows or Linux renderer can use the same bounded glyph order and
  metrics before it chooses an explicit scale, baseline, glyph coverage cache,
  and canvas paint.
- Failure stays closed: a host cannot accidentally paint a partial value or
  silently substitute an unknown scalar through this API.
- The platform's existing GDI bridge remains temporary; this decision does not
  claim a native surface has switched to the new path.

## Deliberately absent

- kerning, ligatures, shaping, script handling, bidirectionality, fallback,
  wrapping, selection, editing, and a public application text capability;
- font source selection, caching, outline loading, rasterization, or native
  drawing;
- a default typeface, bundled font asset, or any desktop-host integration.

## Alternatives considered

**Ask GDI or another system API to lay out every run.** This preserves current
Windows behaviour but keeps a cross-platform owned renderer dependent on a
host-specific text stack. Deferred only as the current temporary bridge.

**Adopt a general text library.** It would solve substantially more typography
but add a shipped third-party dependency at the platform core. Refused under
Decision 0005.

**Build shaping, fallback, and wrapping now.** Each has independent language,
memory, and observable-behaviour questions. Refused until a real application
sets their requirements.

## Revisit conditions

Revisit before a face source, glyph cache, device placement, paint integration,
kerning, shaping, fallback, line layout, editing, or a public text protocol.
