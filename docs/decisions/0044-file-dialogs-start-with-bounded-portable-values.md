# Decision 0044: File dialogs start with bounded portable values

**Status:** Accepted

**Date:** 2026-08-01

## Decision

Anodrel begins file dialogs with strict filter and selected-path values that do
no I/O and expose no native handle or dialog flag. A future host chooses the
dialog, current owner window, and initial directory; applications receive only
the selected bounded path or cancellation behind a separate capability.

## Consequences

- native dialog adapters can share one testable portable contract;
- raw common-dialog filter strings and ambient initial directories cannot reach
  an application-facing surface; and
- opening a file remains distinct from gaining permission to read it.

## Revisit conditions

Revisit before adding a native adapter, a protocol operation, file access,
multiple selection, directories, saving, or a new filter syntax.
