# Selection-scoped file access

**Status:** The portable bounded, one-use selection-reference registry, direct
Windows retained regular-file registry, and Windows UI-session capture path are
implemented. Protocol 1.9 `dialog.open_file.v2` and `file.read_text` are
available only when a host explicitly wires that session-bound capture path;
the default host services remain unavailable. The separately scoped Protocol
1.17 text-write and Protocol 1.22 binary-write contracts are implemented
through their own retained-output-object services; neither extends a read-side
selection reference.

## Purpose

Anodrel will let an authenticated application consume data from one file the
user selected in a host-owned picker, without granting arbitrary filesystem
access. A path returned by `dialog.open_file` is display data only. It is never
a future read request parameter or a capability.

## Protocol boundary

The first read operation uses a host-created **selection reference**:

~~~text
dialog.open_file.v2 -> user chooses one file -> host retains a selection
file.read_text       -> application supplies the opaque selection reference
~~~

The reference is an unguessable, session-bound value. It is not a path, native
handle, directory scope, capability declaration, or durable identifier. It is
invalid outside its authenticated session and expires when that session ends.
The host keeps the resolved selection private and accepts no caller-supplied
path, filename, initial directory, filter, or file identity for a read.

On Windows, the direct adapter generates each Version 1 reference from 128 bits
of CNG random data and encodes it as exactly 22 unpadded base64url characters.

`file.read_text` requires a distinct host-issued `file.read_text` grant and
accepts exactly one selection reference. Protocol 1.9 limits the public result
to 8 KiB of strict UTF-8 source text and defines the safe failures
`file.unavailable`, `file.text_too_large`, and `file.text_invalid`; it exposes
no native status, canonical paths, directory contents, file metadata, or a
general file handle. See `docs/PROTOCOL.md` and Decision 0050.

## Native identity requirement

Each adapter must bind a chosen file to a native read-only file identity. On
Windows the UI-thread picker completion immediately opens and validates the
Windows-confirmed selected regular file through direct APIs before a result
reaches the pipe worker. The retained handle blocks later write, delete, and
rename sharing, rejects directories and reparse points, and records the file
identity the later read consumes. A protocol path, a separately supplied path,
or a later general filesystem reopen is insufficient: replacement, reparse,
and rename races after capture fail safely rather than cause the host to read a
different file.

The selection reference therefore resolves to host-retained native state, not a
saved path string. The portable core may track lifetime and bounded references,
but only an operating-system adapter may read bytes. A capture request shares
the existing one-request UI-thread dialog mailbox with ordinary open and save
pickers, so it cannot introduce concurrent modal UI or a second request queue.

The Windows adapter's per-session registry pairs each CNG reference with the
opened file object. It consumes a reference once and closes all remaining
objects on session cleanup.

The portable `FileTextService` interface accepts only that opaque reference
and has a fail-closed unavailable implementation for hosts that do not wire
selected-file reads. The Windows implementation is thread-safe for one
authenticated session and consumes its retained object before reading.

## Limits and deferred work

The native text reader is currently limited to **32 KiB** of bytes, requires
strict UTF-8, and reads only the retained regular-file handle. Public Protocol
1.9 exposure will apply its stricter 8 KiB response bound. A selection reference
is single-use and the portable store
holds at most 32 live references per session. Binary reads, directories,
multiple selection, persistent grants, bookmarks, drag-and-drop, and
cross-session sharing remain deferred. Text and bounded binary writes have
their own retained-output-object contracts rather than extending a read-side
selection reference; see `docs/FILE_WRITE.md`, `docs/FILE_BINARY_WRITE.md`,
and Decisions 0079 and 0087.

## Development verification

The Windows host includes a development-only UI-session sample that requests a
Protocol 1.9 selection, then consumes its reference once when the user confirms
a small UTF-8 text file:

~~~text
native\\target\\release\\anodrel-windows-host.exe --sample-ui-file-text-client <node.exe> apps\\sample\\dist\\native-client.js
~~~

Cancelling the picker also completes the sample safely. This is a diagnostic
session path, not product executable trust, a persistent file permission, or a
general filesystem bridge.

The separate write diagnostic opens the host-owned save picker, captures one
output object, and writes a fixed line through one save reference. Choose a new
temporary `.txt` destination, then inspect it after the session closes:

~~~text
native\\target\\release\\anodrel-windows-host.exe --sample-ui-file-write-client <node.exe> apps\\sample\\dist\\native-client.js
~~~

It cannot write a later path or reuse the reference; the legacy save diagnostic
remains selection-only.

The separate binary diagnostic uses that same one-use output-object capture
but requires its own `file.write_binary` grant and writes only a fixed 9-byte
sequence. See `docs/FILE_BINARY_WRITE.md` for its command and manual check.

## Security invariants

- A caller cannot select an arbitrary path for `file.read_text`.
- A path returned from Protocol 1.7 is not sufficient to read a file.
- An open-file capability and a read capability are separate grants.
- Session shutdown revokes every selection reference.
- The host never logs a selected path, file contents, native handle, or native
  failure detail through the protocol.
