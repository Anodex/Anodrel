# Decision 0215: Windows owned-text comparison stays fixed and local

- Status: Accepted
- Date: 2026-09-06

## Context

The fixed `ANODREL` report measures one regular selected face's advance and
cache reuse. It cannot show whether the first-party parser, run builder, glyph
cache, and canvas compositor stay geometrically close to the GDI coverage
currently visible on representative fixed Windows-host labels. Replacing the
painter now would still lose shaping, fallback, and hinting without proving a
better result.

An evidence step is needed before a later visual lab can responsibly compare
the two routes. It must not turn application text, font selection, image
output, or an unbounded benchmark into a new host surface.

## Decision

The Windows host provides one no-argument local-only
`--owned-text-comparison-report` diagnostic. It uses these four exact static
labels already present on first-party Windows surfaces:

| Label | Em pixels | Weight |
| --- | ---: | ---: |
| `Native Application Platform` | 15 | 400 |
| `Startup Lab` | 15 | 500 |
| `Validated Application` | 17 | 400 |
| `Anodrel` | 24 | 500 |

Every case has zero tracking. For the exact fixed size and weight, the report
asks GDI to produce the current host coverage and separately obtains the same
selected face bytes for the owned parser. The owned route builds one unshaped
run, composes it through one face-local glyph cache, and draws it into a
private canvas. It measures each route's advance, non-background coverage,
and the two coverage regions' alpha overlap after aligning their ink bounds.
The alignment intentionally measures glyph shape rather than claiming that a
future baseline policy already exists.

The command opens no window and accepts no input, path, font, application
value, or configuration. It writes one closed JSON record to standard output
and retains no face, glyph cache, image, timing, or pixel data after the
process exits. A failed selected source, parser, run, metric, glyph, or empty
coverage stops the entire report without partial output.

The private report vocabulary is:

| Field | Meaning |
| --- | --- |
| `benchmark` | Always `anodrel.host.owned-text-comparison.v1`. |
| `caseCount` | Number of fixed comparison cases. |
| `maxAdvanceDriftMilliPixels` | Greatest absolute GDI/owned advance difference. |
| `minCoverageOverlapMilli` | Lowest per-case aligned alpha overlap, in thousandths. |
| `gdiInkPixels` | Total non-background coverage pixels from the GDI route. |
| `ownedInkPixels` | Total non-background coverage pixels from the owned route. |
| `retainedMaskCount` | Total masks retained while each fixed case was composed. |
| `retainedPixelCount` | Total retained mask pixels while each fixed case was composed. |
| `scope` | Fixed local, non-release boundary statement. |

The report is evidence, not a quality threshold, performance benchmark,
accessibility check, or release gate. Its values must not be compared across
machines. It does not replace the existing GDI painter or change its
measurement, wrapping, paint, accessibility, or cache path.

## Consequences

- The next text decision gains repeatable coverage and metric facts for fixed
  regular and medium host labels instead of extrapolating from one word.
- The selected-face source now proves the exact GDI selection used by each
  fixed comparison case without a filesystem font dependency or third-party
  runtime.
- A later visible comparison must define baseline alignment, retained source
  and cache lifetime, clipping and invalidation, shaping and fallback,
  accessibility semantics, manual visual acceptance, and whole-frame
  measurement before it can change a released text route.

## Deliberately absent

- arbitrary text, face, size, weight, tracking, output path, image export,
  persistent records, background sampling, or telemetry;
- visual output, a native window, a protocol field, an application capability,
  or an accessibility-provider change; and
- a quality score, pass/fail threshold, visible painter selection, fallback,
  shaping, hinting, or Linux/macOS adapter.

## Alternatives considered

**Switch the fixed labels to the owned route.** The current run is unshaped and
unhinted, and no visual or whole-frame proof exists. Refused.

**Compare arbitrary strings and system fonts.** That would add an input,
font-selection, and work-amplification boundary merely to produce local
diagnostics. Refused.

**Write screenshots for external comparison.** Persistent pixel data adds path,
lifetime, privacy, and cleanup concerns without yet proving a production
renderer. Refused.

## Revisit conditions

Revisit before changing the cases, diagnostic vocabulary, selected-face
lifetime, visibility, quality criteria, or release status. A visible owned
text route still requires the additional evidence listed above.
