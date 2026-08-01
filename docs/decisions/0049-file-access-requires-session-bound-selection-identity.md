# Decision 0049: File access requires session-bound selection identity

**Status:** Accepted

**Date:** 2026-08-01

## Context

Anodrel's open picker currently returns a selected path as application data.
Adding a read operation that accepts that path would let a caller transform any
absolute string into filesystem authority and would also be vulnerable to a
file being replaced after selection.

## Decision

The first file-read protocol will not accept a path. A future
`dialog.open_file.v2` will return a host-created, unguessable selection
reference in addition to any display-safe result. The same authenticated
session can present that reference to a distinct capability-checked
`file.read_text` operation. The host retains the selected file's native,
read-only identity; the reference is session-bound and is revoked at session
shutdown.

No file read operation may be implemented until the Windows adapter proves
regular-file validation and identity-preserving access across replacement,
rename, and reparse-point races. The portable core may own bounded reference
lifetime, but it must not perform filesystem I/O.

## Consequences

- Existing Protocol 1.7 paths never grant file read access.
- File selection and file reading remain independently granted.
- A future adapter requires a narrowly scoped native handle boundary rather
  than a general path-based filesystem API.
- Reads and writes can be designed independently with their own race and
  recovery policies.

## Revisit conditions

Revisit before adding binary reads, writes, persistent selection grants,
multiple selection, drag-and-drop, directory access, non-Windows adapters, or
sharing a selection between sessions.
