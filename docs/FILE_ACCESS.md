# Selection-scoped file access

**Status:** A portable bounded, one-use selection-reference registry and a
direct Windows regular-file identity capture are implemented; no file read or
write protocol operation exists yet.

## Purpose

Anodrel will let an authenticated application consume data from one file the
user selected in a host-owned picker, without granting arbitrary filesystem
access. A path returned by `dialog.open_file` is display data only. It is never
a future read request parameter or a capability.

## Planned boundary

The first read operation will use a host-created **selection reference**:

~~~text
dialog.open_file.v2 -> user chooses one file -> host retains a selection
file.read_text       -> application supplies the opaque selection reference
~~~

The reference is an unguessable, session-bound value. It is not a path, native
handle, directory scope, capability declaration, or durable identifier. It is
invalid outside its authenticated session and expires when that session ends.
The host keeps the resolved selection private and accepts no caller-supplied
path, filename, initial directory, filter, or file identity for a read.

`file.read_text` will require a distinct host-issued `file.read_text` grant and
will accept exactly one selection reference. It will return bounded UTF-8 text
or a stable safe failure category; it will not expose native status, canonical
paths, directory contents, file metadata, or a general file handle.

## Native identity requirement

Before the operation is implemented, each adapter must bind a chosen file to a
native read-only file identity. On Windows this means opening and validating
the selected regular file through direct Windows APIs while preserving an
identity the later read can verify. Reopening an arbitrary path after selection
is insufficient: replacement, reparse-point, and rename races must fail safely
rather than cause the host to read a different file.

The selection reference must therefore resolve to host-retained native state,
not merely a saved path string. The portable core may track lifetime and
bounded references, but only an operating-system adapter may read bytes.

## Limits and deferred work

The first text reader must define a strict byte limit below the 64 KiB wire
frame, UTF-8 behavior, regular-file checks, cancellation, and expiry behavior
before code lands. A selection reference is single-use and the portable store
holds at most 32 live references per session. Binary reads, writes,
directories, multiple selection, persistent grants, bookmarks, drag-and-drop,
and cross-session sharing remain deferred.

## Security invariants

- A caller cannot select an arbitrary path for `file.read_text`.
- A path returned from Protocol 1.7 is not sufficient to read a file.
- An open-file capability and a read capability are separate grants.
- Session shutdown revokes every selection reference.
- The host never logs a selected path, file contents, native handle, or native
  failure detail through the protocol.
