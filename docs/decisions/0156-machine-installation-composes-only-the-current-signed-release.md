# Decision 0156: Machine installation composes only the current signed release

**Status:** Accepted

**Date:** 2026-08-31

## Context

Anodrel already has independently checked boundaries for a signed current
release, a fixed machine root, private staging, staged executable verification,
no-overwrite promotion, and fixed policy publication. Leaving their composition
to a future command would invite callers to choose paths or omit a gate.

## Decision

Provide one owned installation transaction with no arguments. It activates the
current signed resource-bearing installer to select the signed application
identity and requires that the fixed machine policy record be absent. An
existing selected record must use the separate update path, never this initial
install route. The transaction then derives its internal machine root and runs
the existing preparation gate again. Only that prepared release may promote,
and only that promoted release may publish the fixed machine record.

The transaction does not expose its root, record, executable, manifest,
publisher, or payload. It creates no trust, process, user data, updater,
shortcut, service, association, or network connection. The command-line tool
does not invoke it until elevation, user-facing reporting, and signed-fixture
acceptance have their own review.

## Consequences

- The installer cannot install a sidecar release or a caller-selected path.
- A selected policy prevents this initial-install route from publishing an equal
  or older signed release over it.
- The current installer signature is checked before root selection and again at
  preparation, so composition does not weaken either gate.
- A failed publication leaves an unselected complete version and the previous
  selected policy intact; it does not overwrite or delete content as rollback.
- Runtime tests can prove an unsigned current image stops before machine-root
  selection without modifying Program Files.

## Revisit conditions

Revisit for an approved installer command UX, a transactional policy store, a
brokered deployment service, or an installer architecture other than the
current signed executable.
