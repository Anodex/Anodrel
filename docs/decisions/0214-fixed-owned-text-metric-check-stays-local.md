# Decision 0214: Fixed owned-text metric check stays local

- Status: Accepted
- Date: 2026-09-05

## Context

Decision 0213's fixed local report records the current GDI width and the owned
run width for the same selected face, literal, and pixel size. Its existing
host test proved only that both values were positive. A scale, design-unit, or
placement regression could therefore produce a plausible report while moving
the owned run materially away from the current route.

The report cannot become a broad typography or performance gate: it has one
unshaped ASCII literal, GDI rounds to whole pixels, and the owned path has not
earned shaping, hinting, visual, accessibility, or whole-frame equivalence.

## Decision

Keep the report vocabulary and fixed input unchanged. The Windows-host test
requires the absolute difference between the GDI rounded advance and the owned
advance to be at most 1,000 milli-pixels (one physical pixel):

~~~text
abs(gdiAdvancePixels * 1,000 - ownedAdvanceMilliPixels) <= 1,000
~~~

The allowance deliberately covers GDI's whole-pixel rounding while detecting a
meaningful fixed scale or placement error. It is evaluated only inside the
existing selected-face host test; no command, report field, protocol, visible
surface, or release gate changes.

## Consequences

- The known selected-face integration path now detects an advance regression
  that positive-value checks would miss.
- The check retains Decision 0213's closed, no-input diagnostic boundary and
  does not retain a face or cache beyond the test invocation.
- It makes no claim about quality, shaping, hinting, visual parity,
  accessibility parity, startup cost, frame time, or cross-machine performance.

## Alternatives considered

**Require exact equality.** GDI intentionally reports a rounded whole-pixel
metric while the owned route records thousandths of a pixel. Exact equality
would turn a valid representation difference into a brittle test. Refused.

**Add arbitrary text, faces, or sizes to find more mismatches.** That would
widen a local diagnostic into an input and work-amplification boundary before a
visible owned route has a contract. Refused.

**Treat this as painter-quality acceptance.** One unshaped ASCII run cannot
prove general typography, visual parity, accessibility, or frame performance.
Refused.

## Revisit conditions

Revisit before changing the fixed face, literal, size, tolerance, report
schema, or visibility. A visible owned painter still requires the full evidence
listed in Decision 0213: source lifetime, clipping and invalidation, shaping
and hinting, accessibility agreement, visual comparison, and whole-frame
performance measurement.
