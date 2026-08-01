# Anodrel file-dialog foundation

**Status:** Portable open-dialog values, a direct Windows adapter, and a
bounded UI-thread request bridge are implemented. Protocol 1.7 exposes the
session-bound `dialog.open_file` capability through that bridge.

## Boundary

The first file-dialog contract models one host-owned open-file choice. An
application supplies no native window handle, initial directory, raw filter
string, file-system path, multiple-selection flag, save location, or dialog
flag. Hosts later select a documented dialog configuration and return either a
bounded selected path or cancellation.

The application or pipe worker never invokes a native dialog. A
`FileDialogMailbox` holds at most one request, lets the host UI thread take it,
and waits only for that UI thread to complete or safely fail it. It times out
after two minutes and has no queue or history. A selected path remains data; it
does not grant file read, write, enumeration, handle access, or process launch.

## Portable values

- A filter has a visible ASCII label and one to eight lowercase extensions.
- Labels are at most 64 bytes; extensions are at most 16 bytes and contain
  only lowercase ASCII letters and digits, without dots or wildcards.
- A selected path is an absolute non-empty path at most 32 KiB in UTF-8 bytes.
  It is opaque application data, never a native handle or permission grant.
- The portable crate performs no filesystem I/O, path resolution, dialog call,
  logging, or protocol operation.

## Deferred

Initial-directory policy, file access, save dialogs, folder dialogs, multiple
selection, confirmation UI, and non-Windows adapters need separate decisions.
