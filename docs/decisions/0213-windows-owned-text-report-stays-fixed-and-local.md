# Decision 0213: Windows owned-text report stays fixed and local

- Status: Accepted
- Date: 2026-09-05

## Context

Decision 0212 proves that one fixed selected Windows face can pass through the
owned parser, unshaped run builder, glyph-mask cache, and offset compositor.
That proof does not show the local measurements needed before considering a
visible-painter comparison: the current GDI route's advance, the owned run's
advance, the actual bounded cache retention, or the first-pass and cache-reuse
costs on the same machine.

Those facts must be measurable without adding an application text API, a font
selection API, a persistent benchmark database, or a second visible text
surface. The existing GDI painter remains responsible for the released host's
shaping and hinting until a later comparison can support a different decision.

## Decision

The Windows host provides one no-argument, local-only
`--owned-text-report` diagnostic. It always composes the literal `ANODREL` at
32 physical pixels in the host's fixed selected GDI face. It opens no window,
accepts no input or path, changes no policy or application state, starts no
session or child, and writes no file, log, network request, or telemetry.

One report constructs the selected face under Decision 0211's existing source
bound, parses it, builds the existing single-line run, and composites that run
twice into the existing fixed internal canvas. It measures the first and second
row separately; the second row differs only by whole-pixel translation and
therefore exercises the existing exact mask-cache reuse rule. It also reads the
current GDI route's width for that same fixed face, text, and pixel size, but
does not ask GDI to paint a comparison image.

On success, standard output contains one JSON object with this exact private
diagnostic vocabulary:

| Field | Meaning |
| --- | --- |
| `benchmark` | Always `anodrel.host.owned-text.v1`. |
| `sourceBytes` | Bounded copied selected-face byte count. |
| `glyphCount` | Source-order glyph count in the fixed owned run. |
| `gdiAdvancePixels` | Current GDI-route advance, rounded to whole pixels. |
| `ownedAdvanceMilliPixels` | Owned-run advance rounded to thousandths of a pixel. |
| `retainedMaskCount` | Masks retained after the two rows. |
| `retainedPixelCount` | Total retained coverage pixels after the two rows. |
| `firstRowMicroseconds` | Local elapsed time for the first row's cache lookup and composite work. |
| `reusedRowMicroseconds` | Local elapsed time for the whole-pixel-shifted second row's cache lookup and composite work. |
| `scope` | Fixed statement of the diagnostic's local, non-release boundary. |

These fields are diagnostic output, not protocol fields, release requirements,
or a quality threshold. A report observes one machine and one process timing;
it does not establish a performance claim and must never be compared across
machines as a release benchmark.

## Consequences

- The next owned-text decision has reproducible local evidence about metrics,
  bounds, cache reuse, and composition cost for the actual selected Windows
  face.
- The command adds no ambient authority, application-visible state, or user
  interface. It can be run during development or release investigation without
  affecting an installed application.
- A bad source, parser, run, glyph, or canvas result fails closed without a
  partial report.
- The current GDI route is still the only visible Windows text painter. The
  report does not claim shaping, hinting, visual equivalence, accessibility
  equivalence, startup impact, or frame-budget parity.

## Alternatives considered

**Replace the painter now.** The owned run remains intentionally unshaped and
unhinted. Replacing GDI before a quality and frame comparison would make a
visible surface worse without evidence. Refused.

**Expose arbitrary report text or font settings.** That would create a new
input, font-selection, work-amplification, and observability boundary for a
diagnostic. The fixed source is sufficient for the decision it supports.

**Persist repeated samples.** Persisted local performance data would introduce
paths, state retention, lifecycle, and privacy questions while still not prove
cross-machine performance. One explicit local reading is simpler and honest.

## Revisit conditions

Revisit this boundary only when a proposed visible owned-text route has a
defined retained-face lifetime, clipping and invalidation policy, shaping and
hinting plan, accessibility agreement, visual-comparison procedure, and
whole-frame performance measurement.
