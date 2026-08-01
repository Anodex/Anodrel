# Anodrel file-dialog foundation

**Status:** Portable open-dialog filter and selected-path values are implemented;
the native Windows dialog and protocol capability remain deferred.

## Boundary

The first file-dialog contract models one host-owned open-file choice. An
application supplies no native window handle, initial directory, raw filter
string, file-system path, multiple-selection flag, save location, or dialog
flag. Hosts later select a documented dialog configuration and return either a
bounded selected path or cancellation.

## Portable values

- A filter has a visible ASCII label and one to eight lowercase extensions.
- Labels are at most 64 bytes; extensions are at most 16 bytes and contain
  only lowercase ASCII letters and digits, without dots or wildcards.
- A selected path is an absolute non-empty path at most 32 KiB in UTF-8 bytes.
  It is opaque application data, never a native handle or permission grant.
- The portable crate performs no filesystem I/O, path resolution, dialog call,
  logging, or protocol operation.

## Deferred

The Windows Common Dialog adapter, session-bound `dialog.open_file` capability,
initial-directory policy, file access, save dialogs, folder dialogs, multiple
selection, confirmation UI, and non-Windows adapters need separate decisions.
