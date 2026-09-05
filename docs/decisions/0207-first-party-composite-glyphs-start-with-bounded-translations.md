# Decision 0207: First-party composite glyphs start with bounded translations

- Status: Accepted
- Date: 2026-09-05

## Context

The owned font path can map characters, read metrics, extract simple TrueType
outlines, turn them into exact quadratic paths, and create bounded masks. Many
ordinary font glyphs nevertheless consist of component outlines: for example,
a base glyph plus one or more accents. Leaving every composite as an error
keeps the Windows GDI bridge in place for common text and prevents the portable
path from covering its next practical class of geometry.

Composite components can also recurse, reference their parents, attach one
component point to another, apply fractional affine transforms, carry hinting
instructions, and alter metrics. Treating all of that as a minor extension to
simple points would either lose precision or make the geometry parser
unbounded.

## Decision

Extend `anodrel-font` to flatten the first safe composite subset into its
existing owned `GlyphOutline`:

- a composite uses the standard `-1` contour-count marker;
- each component names an in-range glyph and supplies signed x/y translation
  arguments, in byte or word form;
- nested components are allowed to a fixed depth of 16, must be acyclic, and
  may contain at most 128 component records per composite;
- the fully expanded result remains within the existing 4,096-contour and
  16,384-point limits, and every translated coordinate must remain a signed
  16-bit design-unit value; and
- composite instruction bytes are range-checked and ignored, exactly like
  simple-glyph instructions.

The composite's header supplies the returned outline bounds. Its `hmtx` record
continues to supply its metrics: `USE_MY_METRICS`, overlap hints, and
round-to-grid hints do not alter an unhinted geometry result.

Point-to-point attachment and all affine component transforms remain explicit
closed outcomes. They can require fractional coordinates beyond the current
exact doubled-unit path format, and a later decision must establish their
precision, offset rules, and output contract before they are accepted. Reserved
flags, malformed component records, trailing non-padding bytes, an invalid
first component placement, cycles, over-depth graphs, and over-limit expanded
geometry are also closed outcomes.

## Consequences

- The portable owned text foundation can represent common translated composite
  glyphs without a toolkit, system font wrapper, or third-party runtime.
- Every component remains parsed through the same validated `loca` index and
  simple-outline decoder; there is no alternate unchecked font-byte route.
- The existing quadratic converter and glyph cache need no special composite
  behavior because they receive one normal flattened outline.
- Transform-heavy and point-attached composite glyphs remain visibly
  unsupported rather than rounded, partially placed, or silently substituted.

## Deliberately absent

- component scale, rotation, shear, scaled-offset policy, point attachment,
  phantom points, hinting, grid fitting, variation deltas, and metric override;
- kerning, shaping, fallback, line layout, face discovery, application font
  input, host integration, and a public application text capability; and
- any operating-system API, file access, global cache, or third-party font
  dependency.

## Alternatives considered

**Leave every composite unsupported.** Safest as a parser boundary, but it
excludes a common geometry form and slows the owned text path's progress.
Refused.

**Accept transforms by rounding coordinates to integer design units.** This
would make rendered geometry incorrect before it reaches the exact quadratic
path layer. Refused.

**Adopt a font library.** This would introduce a third-party native runtime at
the core text boundary. Refused under Decision 0005.

## Revisit conditions

Revisit before affine transforms, point attachment, a higher-precision path
representation, component metrics, hinting, variation fonts, shaping, an owned
face source, or a native text painter.
