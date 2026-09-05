# Decision 0210: Windows text coverage stays pixel-bounded

- Status: Accepted
- Date: 2026-09-05

## Context

The Windows host creates an antialiased GDI coverage bitmap for each distinct
cached text run. Counting at most 512 cached keys is insufficient by itself: a
single unusually wide or tall run could allocate a large DIB and coverage vector,
and many ordinary-sized entries could still retain far more memory than a
first-party surface needs.

The portable glyph cache already treats pixel area as its resource unit. The
GDI route should meet the same explicit memory discipline until a later font
source replaces it.

## Decision

One Windows GDI text run may create at most **262,144 coverage pixels** (one MiB
of `f32` coverage). The thread-local text raster cache retains at most **512**
run keys and at most **2,097,152 coverage pixels** (eight MiB). A run outside
the per-run bound produces no mask and paints nothing. Before inserting a new
successful result that would exceed either cache bound, the host clears that
whole cache and starts the next bounded generation; failed rasterizations count
as keys but consume zero coverage pixels.

The bounds are checked before allocating the GDI DIB or Rust coverage vector.
They do not change source text, GDI shaping, line measurement, alignment,
painting, accessibility, or the private single-blit presentation path.

## Consequences

- Host-controlled and externally supplied documents cannot retain unbounded
  GDI coverage memory through distinct text values or extreme layout sizes.
- The cache stays simple, local, and deterministic: dropping or clearing it
  releases all retained coverage together, with no background collector or
  cross-thread state.
- A too-large run visibly omits its text rather than risking an allocation or
  producing partial coverage. The existing UI document size and layout bounds
  normally keep all supported first-party lines below this ceiling.

## Deliberately absent

- a cache readback, statistics API, size configuration, per-entry LRU,
  background eviction, global cache, application control, or cache protocol;
- a new text layout, ellipsis, fallback, font discovery, native error surface,
  or change to the portable glyph cache; and
- changes to GDI shaping, handles, device contexts, bitmap presentation, or
  accessibility semantics.

## Alternatives considered

**Bound only the number of cache entries.** Small entries are cheap but large
ones are not, so that does not establish a memory ceiling. Refused.

**Add an LRU cache with adjustable budgets.** It could preserve more entries,
but adds public policy and mutation complexity before a real application has a
measured reuse workload. Deferred.

**Allow a large run and rely on allocation failure.** An allocation failure is
late, environment-dependent, and can destabilize a surface. Refused.

## Revisit conditions

Revisit before a new font source, a cache statistics or configuration surface,
per-entry eviction, larger document limits, line-layout policy, or host-level
text integration beyond the current GDI route.
