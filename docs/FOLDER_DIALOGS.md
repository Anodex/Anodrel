# Anodrel folder-selection contract

**Status:** The portable value, Protocol 1.28, TypeScript SDK/mock,
installed-policy compatibility, direct Windows picker, UI-thread host routing,
and an explicit manual diagnostic are implemented. The first direct
desktop-picker check remains manual.

## Purpose and boundary

Anodrel's first folder picker is one explicit, user-mediated choice. It lets an
application ask the host to display a native folder dialog and receive either
one absolute filesystem-folder path or cancellation.

The application cannot supply a native window handle, initial folder, dialog
caption, file filter, multiple-selection flag, file-system path, or dialog
option. The host chooses the owner window and native configuration. A selected
folder path is data, not authority: it does not grant enumeration, read,
write, creation, deletion, process launch, a handle, a watcher, or a retained
folder permission.

## Planned protocol surface

Protocol **1.28** adds one operation:

| Field | Value |
| --- | --- |
| Operation | `dialog.open_folder` |
| Payload | `{}` exactly |
| Grant | `dialog.open_folder` |
| Selected result | `{ "status": "selected", "path": string }` |
| Cancelled result | `{ "status": "cancelled" }` |
| Service failure | `dialog.unavailable` |

`path` is an absolute non-empty filesystem path of at most **32 KiB** in UTF-8
bytes. It contains no native handle, object identifier, access mode, metadata,
initial-folder fact, or persistent authorization. A failure never contains a
path or native error detail.

The empty payload is intentional. Filters describe files, not folders; an
initial folder would make an application's ambient filesystem knowledge visible
to the dialog; and a caller-selected dialog option would turn host UI policy
into an application API.

## Host ownership

`FileDialogMailbox` carries at most one modal file or folder request per
session. Only the host UI thread may take a folder request, show its native
dialog, and complete the matching request. The worker waits for at most two
minutes; a second request, expired response, wrong request identity, or result
of the wrong kind is unavailable.

A direct Windows implementation uses the Common Item Dialog in folder mode.
It requests a filesystem result, attaches it to the host-owned window, and
converts the returned shell item into this bounded portable value before the
worker receives it. The COM dialog, shell item, allocated display-name buffer,
and any native failure remain adapter-private.

## What this does not add

- File selection, save destinations, filters, or file read/write authority.
- Folder enumeration, recursive traversal, watchers, drag-and-drop, or
  multiple selection.
- Initial-directory, title, owner-window, native-flag, or current-location
  control or readback.
- A folder reference from the display-only `dialog.open_folder` route. The
  separately implemented `dialog.open_folder.v2` capture route is defined in
  `docs/FOLDER_ACCESS.md` and remains distinct from this path-only operation.
- A background UI route, callbacks, or non-Windows adapter.

Any folder-access operation needs its own capability, retained-identity rules,
threat model, and decision. The existing selected-file and selected-output
boundaries remain unchanged; see `docs/FILE_DIALOGS.md`,
`docs/FILE_ACCESS.md`, `docs/FOLDER_ACCESS.md`, and `docs/FILE_WRITE.md`.

## Verification

Portable tests prove absolute-path validation, exact request/result pairing,
one-request concurrency, and cancellation. The Windows adapter has focused
tests for its exact COM identifiers, folder-only option composition, and
bounded UTF-16 result decoding. Its final manual check must still show the
owned desktop picker, select one filesystem folder, cancel a second run, and
confirm both outcomes close the matching session without granting access.
