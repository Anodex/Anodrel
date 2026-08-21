# Decision 0079: Save-file writes use host-retained objects

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0047 deliberately made `dialog.save_file` a choice only: returning a
path must not create, truncate, or write a file. A later write operation cannot
accept that path, because it would turn any absolute string into filesystem
authority and would reopen a race between picker selection and mutation.

The existing read boundary already proves the appropriate shape: capture one
native object while the host owns the picker, return an opaque session-bound
reference, and consume that object once through a separately granted operation.
Writing needs its own object type and recovery rules, especially when a user
chooses a new filename.

## Decision

Protocol 1.17 will add `dialog.save_file.v2` and `file.write_text`.

- `dialog.save_file.v2` keeps the exact bounded filter request and requires the
  existing `dialog.save_file` grant. A successful result contains the selected
  display path and one host-created `saveReference`; cancellation remains
  explicit. Legacy `dialog.save_file` remains exactly selection-only.
- `file.write_text` requires the independent `file.write_text` grant. It
  accepts exactly a `saveReference` and bounded UTF-8 `text`, consumes the
  reference once, and writes only through its retained native object. It never
  accepts a path, filename, directory, native handle, encoding, offset,
  length, append flag, overwrite flag, or atomicity option.

A Version 1 save reference is exactly 22 unpadded base64url characters derived
from 128 bits of host CNG randomness. It is opaque, session-bound,
non-durable, and distinct from an open-file `selectionReference` both in the
protocol and portable types.

The Windows adapter captures an existing destination with a write handle that
permits later readers but blocks writers, rename, and deletion. It validates
the opened object as a non-directory, non-reparse regular file before it can
be registered. It does not truncate that file at capture time.

For a destination that did not exist at the moment Windows reported the
selection, the adapter may create exactly that name only with a create-new
operation; a creation race fails safely rather than opening the replacement.
The resulting handle is marked for deletion until `file.write_text` begins.
Dropping an unused reference or ending its session therefore removes the
empty newly created object through that handle, without reopening a path.

The first write is one synchronous replacement attempt: seek the retained
handle to zero, write the complete bounded value, truncate any old tail, and
flush before reporting success. It is deliberately **not an atomic
replace**. A native failure after mutation begins can leave a newly created
file or an existing destination empty, partial, or partly replaced; the only
public failure is `file.unavailable`. Applications that require crash-safe
atomic replacement must wait for a separately designed transaction contract.

The input text limit is 8 KiB of UTF-8 source bytes. It leaves bounded headroom
for JSON escaping in Wire 1.0's 64 KiB frame. Cancellation is observed only
before the operation starts; a started write completes synchronously and
returns its normal success or failure. The protocol exposes no write progress,
file metadata, durability guarantee, native error, or path in diagnostics.

Installed application record version 1.7 adds `file.write_text` as a new
optional grant. Earlier records must reject it, so updating a host cannot widen
an existing application's authority.

## Consequences

- A save path returned by Protocol 1.8 still grants no write authority.
- The same reference cannot be used for a read, replayed, shared across
  sessions, or retained after shutdown.
- The portable core owns validation, capability checks, and reference lifetime;
  only the Windows adapter performs filesystem I/O.
- The first API offers exact bounded text replacement, not binary output,
  append, streaming, directory access, write progress, readback, or atomic
  replacement.

## Revisit conditions

Revisit before adding atomic replacement, binary writes, appending, partial or
streaming writes, a durability result, persistent grants, multiple files,
directory access, another operating-system adapter, or any recovery API.
