# Decision 0002: Windows is the first supported operating system

**Status:** Accepted

**Date:** 2026-07-31

## Context

Anodrel needs a small first scope that can be tested against one concrete
operating system before its service contracts are generalized. The project is
being initiated on Windows, and the roadmap already identifies Windows as the
expected first host.

## Decision

The first native Anodrel host will support Windows. macOS and Linux remain
future adapters behind the same platform service contracts.

The current protocol and mock host remain transport-neutral and are not an
implementation commitment to any Windows UI or host framework.

## Consequences

Positive:

- host behavior, manual tests, and security assumptions have one initial OS
  target;
- native design choices can be validated without simulating cross-platform
  parity prematurely;
- future adapters have a documented reference contract to meet.

Tradeoffs:

- Windows behavior must not leak into cross-platform protocol types;
- macOS and Linux support remains unavailable until separately designed and
  tested.

## Revisit conditions

Revisit if the initial product requirement changes to require another operating
system before a Windows host can be validated.
