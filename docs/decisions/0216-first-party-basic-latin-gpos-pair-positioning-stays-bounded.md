# Decision 0216: First-party basic-Latin GPOS pair positioning stays bounded

- Status: Accepted
- Date: 2026-09-06

## Context

Decision 0208 added the legacy OpenType `kern` table's conventional pair
adjustments. The first fixed Windows comparison records up to 3.783 physical
pixels of advance drift from the current GDI result across static labels. Modern
Windows fonts often carry their ordinary Latin kerning in the OpenType GPOS
`kern` feature instead of the legacy table, so the existing first-party run
cannot observe that positioning.

Using the Windows text layout engine would conceal the gap behind an operating
system dependency. Treating all GPOS as a general shaping engine would be
misleading and far too broad: GPOS includes contextual positioning, marks,
device corrections, script selection, and variation behavior that the current
single-line source-order run deliberately does not model.

## Decision

`anodrel-font` may parse one optional, already-owned OpenType GPOS version-1.0
table beside a complete validated horizontal-metric source. It considers only a
`kern` feature selected from the `latn` script's default language system. This
feature is used only when `anodrel-text` is building an all-ASCII run; every
other run retains Decision 0208's legacy `kern` behavior. That scope makes the
script choice explicit instead of accidentally applying Latin positioning to
unrelated text.

The source is bounded to two MiB. It validates every offset, count, selected
feature and lookup reference before a lookup can use it. At most 32 selected
pair-positioning lookups may be retained. A selected lookup contributes only
when it has lookup type 2, or a version-1 extension lookup type 9 that resolves
to type 2. Its flags must be either zero or the `IgnoreMarks` bit alone; the
latter is equivalent for the all-ASCII source that may use this path. It then
accepts pair-positioning format 1 or format 2 with a first value record
containing only an x-advance adjustment and an empty second value record. All
other valid positioning forms are ignored, not approximated. Malformed selected
structures reject the face; a valid but unsupported positioning form contributes
zero.

Format 1 uses a validated coverage table and sorted second-glyph pairs. Format
2 uses validated coverage and class definitions with a bounded rectangular
class-record matrix. Both return only one signed horizontal adjustment in font
design units. Each lookup is queried without allocation. If an applicable GPOS
source exists, its adjustments replace rather than add to the legacy `kern`
result; otherwise the legacy result remains the fallback. `anodrel-text`
continues to enforce its existing signed pen-position limit after every
adjustment.

## Consequences

- Fixed first-party Windows labels can use the same modern conventional Latin
  pair-positioning source their selected Windows face provides, without a
  third-party runtime or a system layout call.
- The parser stays a borrowed, bounded, unsafe-free portable component; the
  Windows selected-face adapter stays the only operating-system source.
- The fixed comparison can measure whether this narrow standard feature reduces
  advance drift before any visual painter, baseline policy, or release claim is
  changed.

## Deliberately absent

- non-Latin script selection, language-system selection, GPOS version 1.1,
  feature variations, `dist`, mark positioning, cursive positioning, contextual
  positioning, device or variation adjustments, cross-stream placement, and
  glyph substitution;
- fallback, ligatures, bidirectionality, combining marks, line breaking,
  editing, native text APIs, application font control, or a public text API;
- any visible-painter, accessibility, protocol, Linux, macOS, font discovery,
  cache, or release change.

## Alternatives considered

**Use DirectWrite or GDI layout for the owned run.** It would improve one host's
spacing but would leave the portable renderer's output dependent on a hidden
platform layout engine. Refused.

**Add all of GPOS.** That would claim shaping support that Anodrel cannot yet
verify for its current document model. Refused.

**Add legacy and GPOS adjustments together.** Fonts may carry equivalent data
in both tables, causing double kerning. GPOS therefore replaces legacy only
when this narrow selected GPOS source applies.

## Revisit conditions

Revisit before changing the source limit, selected script or language system,
lookup or value-format support, legacy fallback rule, text-script scope,
visible text route, or quality/release status.
