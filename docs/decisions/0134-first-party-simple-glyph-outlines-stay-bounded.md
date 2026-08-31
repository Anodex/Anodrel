# Decision 0134: First-party simple glyph outlines stay bounded

- Status: Accepted
- Date: 2026-08-30

## Context

Decision 0133 can map Unicode characters to glyph identifiers, but a glyph ID
does not yet contain geometry that a native Anodrel renderer can cover. The
TrueType `glyf` table describes that geometry using a bounded index (`loca`),
packed repeated flags, relative vectors, simple contours, and recursive
composite components.

Treating a composite as an empty glyph, handing unvalidated table offsets to a
rasterizer, or executing embedded TrueType instructions would each give an
unowned input format authority over rendering. Bringing in a font library
would give up the first-party native text boundary from Decision 0133.

## Decision

Extend `anodrel-font` with a strict optional outline source made from `head`,
version-1.0 `maxp`, `loca`, and `glyf`. A complete source validates every
location against `numGlyphs`, requires ascending offsets ending at the glyph
table length, and performs a constant-time two-offset lookup for one glyph.
A face with no outline tables still supports character-map lookup; a partial
outline table set is malformed.

The first extraction slice returns only simple glyphs. It parses the glyph
header, contour ends, ignored-but-bounded instruction bytes, packed flag runs,
and relative coordinates into one owned point buffer plus contour-end indices.
It keeps on-curve state and font design units exactly as the font stores them,
with limits of 4,096 contours and 16,384 points per extracted glyph. It does
not execute instructions or construct curves.

Composite glyphs are a separate closed outcome. They are not flattened,
partially rendered, or substituted. Supporting them later needs a dedicated
decision for component transforms, point attachment, recursion, instruction
handling, precision, and output bounds.

## Consequences

- The renderer path obtains a first-party geometric source without a toolkit,
  operating-system text API, or third-party runtime.
- Simple outline extraction is auditable: the only heap work is bounded output
  data and a temporary packed-flag buffer; location lookup itself is constant
  time.
- A malformed or too-complex glyph fails closed before it reaches future curve
  conversion or rasterization.
- Text remains incomplete: common accented glyphs can be composite, and no
  simple outline is yet a visible rendered glyph.

## Deliberately absent

- composite components, affine transformations, point attachment, recursion,
  overlap interpretation, and composite instructions;
- TrueType bytecode execution, grid fitting, hinting, phantom points, metrics,
  kerning, shaping, fallback, and line layout;
- curve construction, scan conversion, anti-aliasing, canvas masks, GPU work,
  a Linux application surface, or a public application protocol operation.

## Alternatives considered

**Delegate the outline to a platform text API.** This would hide the native
text boundary behind inconsistent host behavior. Deferred as the existing
Windows bridge.

**Extract simple and composite glyphs together.** Composite geometry needs
transform and recursion policy that cannot be made safe by treating it as a
small extension to simple points. Refused for this slice.

**Skip validation and let a later rasterizer handle bad glyphs.** This couples
untrusted byte parsing to a hot drawing path and turns malformed offsets into
renderer correctness. Refused.

## Revisit conditions

Revisit before composite glyphs, curves, rasterization, metrics, hinting,
shaping, a face source, application-controlled font data, or a Linux text
surface.
