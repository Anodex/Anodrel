# Decision 0050: File text reads use selection references

**Status:** Accepted

**Date:** 2026-08-01

## Context

Decision 0049 establishes that a path selected by an open picker cannot become
file-read authority. The portable reference registry and Windows retained-file
foundation now make it possible to specify the public boundary, but the current
open picker does not yet capture an identity at the moment of selection. The
protocol must make the separation between display data, selection identity, and
read authority explicit before host integration begins.

## Decision

Protocol 1.9 adds two additive operations:

- `dialog.open_file.v2` accepts the existing bounded `filters` payload and
  requires `dialog.open_file`. On success it returns the selected display path
  plus one opaque `selectionReference`; cancellation remains explicit. The host
  may return a selected result only after it has retained the selected regular
  file's native read-only identity for that reference. It returns only
  `dialog.unavailable` when it cannot do so.
- `file.read_text` accepts exactly one `selectionReference` and requires the
  distinct `file.read_text` capability. It consumes the reference once and
  returns only bounded strict UTF-8 text. It never accepts a path, filename,
  handle, directory, encoding, offset, or length.

Version 1 references are exactly 22 unpadded base64url characters derived from
128 bits of host randomness. They are opaque and session-bound; applications
must not parse, transform, persist, or share them. A consumed, unknown,
expired, cross-session, or otherwise unavailable reference maps only to
`file.unavailable`. Oversized content maps to `file.text_too_large`, and
non-UTF-8 content maps to `file.text_invalid`. Native errors, paths, metadata,
and identity details never cross the protocol.

The first public text response is limited to 8 KiB of UTF-8 source bytes. This
leaves safe headroom for JSON escaping within Wire 1.0's 64 KiB frame limit.
`file.read_text` observes a cancellation before it begins. Once a retained-file
read begins, it completes its fixed bounded read and returns its normal result;
it creates no background transfer or retained work after the response.

The legacy `dialog.open_file` remains unchanged in Protocol 1.7 for clients
that require a display path only. It never returns a selection reference and
can never be used with `file.read_text`.

## Consequences

- File choice and content access remain independently grantable.
- The host must implement a selection-time identity capture seam before it can
  expose either new operation; reopening a returned path is forbidden.
- A host that has not implemented that seam remains safely unavailable for
  Protocol 1.9 file operations while preserving Protocol 1.7 behavior.
- Future binary reads, writes, multiple selection, persistent grants, and
  non-Windows adapters require separate protocol and race-handling decisions.

## Revisit conditions

Revisit before increasing the text limit, adding an encoding selector, allowing
partial or streaming reads, adding write access, retaining references across a
session, or adding another operating-system adapter.
