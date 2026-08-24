# Anodrel file-dialog foundation

**Status:** Portable open/save-dialog values, a direct Windows adapter, and a
bounded UI-thread request bridge are implemented. Protocol 1.7 exposes the
session-bound `dialog.open_file` capability and Protocol 1.8 exposes the
independent `dialog.save_file` capability through that bridge. The same
one-request bridge also carries the capture requests required before
`dialog.open_file.v2` can return a read-side selection reference or Protocol
1.17 `dialog.save_file.v2` can return a write-side save reference. Protocol
1.17 text writing and Protocol 1.22 binary writing consume that same reference
through separately granted operations.

## Boundary

The first file-dialog contract models one host-owned open-file or save-file choice. An
application supplies no native window handle, initial directory, raw filter
string, file-system path, multiple-selection flag, save location, or dialog
flag. Hosts later select a documented dialog configuration and return either a
bounded selected path or cancellation.

The application or pipe worker never invokes a native dialog. A
`FileDialogMailbox` holds at most one request of any kind, lets the host UI
thread take it, and waits only for that UI thread to complete or safely fail it.
It times out after two minutes and has no queue or history. A selected path
remains data; it does not grant file read, write, enumeration, handle access,
or process launch. The Windows UI-session host routes open, save, and
selection-capture requests through that one mailbox, selecting the host window
as the native owner. Write capture remains separate from the legacy save
operation: only the v2 route may retain a native output object. See
`docs/FILE_WRITE.md` and `docs/FILE_BINARY_WRITE.md`.

## Portable values

- A filter has a visible ASCII label and one to eight lowercase extensions.
- Labels are at most 64 bytes; extensions are at most 16 bytes and contain
  only lowercase ASCII letters and digits, without dots or wildcards.
- A selected path is an absolute non-empty path at most 32 KiB in UTF-8 bytes.
  It is opaque application data, never a native handle or permission grant.
- A save destination follows the same absolute 32 KiB bound. Selecting it does
  not create, truncate, or write a file.
- The portable crate performs no filesystem I/O, path resolution, dialog call,
  logging, or protocol operation.

## Deferred

Initial-directory policy, multiple selection, additional confirmation UI, and
non-Windows adapters need separate decisions. `docs/FOLDER_DIALOGS.md` defines
the separately implemented one-folder selection contract through the same
mailbox; it does not add folder access or enumeration.
`docs/FILE_ACCESS.md` defines the accepted read-side selection-identity
requirement. `docs/FILE_WRITE.md` and `docs/FILE_BINARY_WRITE.md` define the
separately scoped text and binary write contracts.
