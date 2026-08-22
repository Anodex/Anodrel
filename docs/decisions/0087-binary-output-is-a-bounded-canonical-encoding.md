# Decision 0087: Binary output is a bounded canonical encoding

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0079 established one retained output object and a small UTF-8 text
replacement operation. Binary artifacts—such as an application export—cannot
be accurately represented by that text boundary. Letting an application choose
a path, handle, offset, chunk, MIME type, or alternate wire would turn this
small capability into filesystem or transfer authority.

The authenticated protocol is strict JSON and intentionally has no raw-byte
frame type. A binary extension must preserve that protocol's validation and
size limits without adding a shipped codec dependency.

## Decision

Protocol 1.22 adds the independent `file.write_binary` capability and one
exact `file.write_binary` operation. It accepts a retained `saveReference` and
an unpadded canonical base64url `bytesBase64Url` field that decodes to no more
than 32 KiB. Its only success result is `{ "status": "written" }`.

The first-party portable decoder accepts only the base64url alphabet, rejects
padding, whitespace, malformed lengths, and non-zero unused trailing bits, and
enforces the limit while decoding. Invalid representation is a payload error;
a valid representation above the decoded bound is `file.binary_too_large`.
The protocol never receives a raw-byte frame, path, MIME type, length claim,
handle, offset, append flag, progress channel, or readback request.

The operation consumes the same host-retained output object established by
`dialog.save_file.v2`, so a one-use selection may be used once by the text or
binary writer but cannot be replayed or directed to another path. It requires
its own immediate `file.write_binary` grant. The direct Windows adapter writes
only the already-decoded bounded bytes through that retained handle and retains
Decision 0079's cleanup and non-atomic-replacement rules.

Installed application record version 1.11 adds `file.write_binary` as an
optional grant. Earlier records reject it, preventing a host update from
widening an existing application's output authority.

## Consequences

- The platform gains one small binary export path without gaining arbitrary
  filesystem, stream, network, or raw-protocol authority.
- One byte sequence has one accepted wire spelling, simplifying validation,
  testing, and logging discipline.
- The existing text writer remains UTF-8-only and independently granted.
- The reusable output-object adapter needs one byte-oriented write method but
  never receives protocol text or performs base64 decoding.

## Revisit conditions

Revisit before adding a raw byte wire format, a larger size limit, streaming,
append or offset writes, MIME metadata, atomic replacement, durability
reporting, persistent grants, multi-file output, another operating-system
adapter, or recovery semantics.
