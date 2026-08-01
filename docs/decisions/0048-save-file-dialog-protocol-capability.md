# Decision 0048: Save-file dialogs use a dedicated session capability

**Status:** Accepted

**Date:** 2026-08-01

## Context

Anodrel already has bounded portable file-dialog values, a direct Windows save
picker, and a host UI-thread bridge. An authenticated application needs to ask
for a save destination without gaining authority to write there or reusing the
separate open-file grant.

## Decision

Protocol 1.8 adds `dialog.save_file`. It accepts the same one bounded
structured filter payload as `dialog.open_file`, but requires the independent
host-issued `dialog.save_file` capability immediately before the request enters
the file-dialog service. Its only successful results are cancellation or one
absolute save destination.

The request uses the existing per-session mailbox and is serviced only by the
host UI thread. It carries no window handle, initial directory, raw dialog
flags, filesystem scope, or write instruction. Selecting a destination never
creates, truncates, or writes a file. A host that cannot service the picker
returns only the stable `dialog.unavailable` category.

## Consequences

- Open and save selection can be granted independently to one authenticated
  session.
- A returned save destination is data, not a filesystem capability.
- Existing hosts that do not wire a save picker fail safely through the default
  unavailable service behavior.
- Any future file-write operation must define its own scoped capability,
  overwrite behavior, race handling, and recovery rules.

## Revisit conditions

Revisit before adding write access, save-history behavior, initial-directory
control, multiple selection, directory selection, or a non-Windows adapter.
