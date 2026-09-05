# Decision 0208: First-party pair kerning stays bounded

- Status: Accepted
- Date: 2026-09-05

## Context

The owned text path can now place one source-order glyph run, but it uses only
individual horizontal advances. Ordinary Latin pairs such as `AV` and `To`
therefore retain visibly excessive space even when the already-owned font
contains a conventional pair adjustment. Delegating that small gap to GDI or a
third-party layout engine would make the portable path depend on a hidden
platform text stack.

The legacy OpenType `kern` table contains several formats and direction modes.
It can also combine multiple subtables, use an override rule, or carry records
unrelated to ordinary horizontal placement. Treating it as a general shaping
engine would make the parser and public layout behaviour needlessly broad.

## Decision

`anodrel-font` accepts an optional, already-owned OpenType version-0 `kern`
table only beside a complete validated horizontal-metric source. It validates
the table header, every declared subtable range, reserved coverage bits, and
final zero padding. At most 32 subtables are accepted.

The owned horizontal lookup uses only conventional format-0 subtables: their
horizontal bit must be set and their minimum and cross-stream bits clear. Each
selected subtable has at most 65,535 strictly sorted `(leftGlyph, rightGlyph)`
pairs, all glyph IDs must belong to the face, and its binary-search parameters
and exact byte length must agree with the records. A lookup binary-searches its
validated borrowed pairs without allocating. Multiple matching values add in
table order unless a matching override subtable replaces the accumulated value.
Other valid formats and non-horizontal modes are ignored, not approximated.

`anodrel-text` applies the resulting signed pair adjustment between the prior
glyph advance and the next glyph position. Both positions and final run advance
are signed design units. Every intermediate and final pen position must remain
within plus or minus 1,048,576 design units; a request exceeding that envelope
returns no partial run.

## Consequences

- Simple Latin text can use the face's conventional pair spacing with no font
  loader, system layout call, global cache, or shipped dependency.
- Absence of `kern`, unmatched pairs, and valid irrelevant subtables all mean a
  deterministic zero adjustment.
- The run remains source ordered and one-scalar-to-one-glyph. Pair kerning does
  not imply ligatures, contextual positioning, marks, script shaping, fallback,
  bidirectionality, wrapping, or a public application text API.

## Deliberately absent

- GPOS, class kerning, device variation, vertical kerning, cross-stream
  placement, minimum kerning, and every `kern` format other than format 0;
- glyph substitution, ligatures, contextual shaping, mark positioning, script
  handling, bidi ordering, line layout, editing, font discovery, and native
  painting; and
- an operating-system text API, application-supplied font bytes, a global font
  cache, or a third-party font dependency.

## Alternatives considered

**Use GPOS or a system text layout API now.** Both can supply richer typography,
but GPOS requires a substantially wider positioning model while a system API
would leave portable output dependent on one host. Deferred.

**Ignore every pair adjustment until full shaping exists.** This keeps the
smallest parser, but leaves common unshaped Latin text visibly worse while the
bounded conventional table is already available. Refused.

**Adopt a general text library.** This conflicts with the first-party runtime
policy in Decision 0005. Refused.

## Revisit conditions

Revisit before GPOS, another `kern` format, device variation, vertical or
cross-stream placement, glyph substitution, script shaping, line layout,
font-source discovery, a painter, or a public text capability.
