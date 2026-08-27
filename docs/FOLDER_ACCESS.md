# Selection-scoped folder-entry access

**Status:** The portable contract and service foundation are implemented. The
protocol operation, native adapter, and host route are not yet implemented.

## Purpose

`dialog.open_folder` returns display data, not filesystem authority. This
separate contract defines the smallest useful follow-on: after a person selects
one folder, an authenticated application may consume one opaque, session-bound
reference to receive a bounded snapshot of that folder's immediate entries.

It is deliberately not a general directory API. The application cannot supply
a path, retain a folder grant, enumerate recursively, select a child, request a
continuation, choose an ordering, follow a link, read file content, write,
delete, create, rename, watch, or learn native metadata.

## Planned protocol surface

Protocol **1.29** will add two operations:

| Operation | Payload | Grant | Successful result |
| --- | --- | --- | --- |
| `dialog.open_folder.v2` | `{}` exactly | `dialog.open_folder` | selected display path plus `folderReference`, or cancellation |
| `folder.read_entries` | `{ "folderReference": string }` | `folder.read_entries` | one bounded entry snapshot |

`dialog.open_folder.v2` is a separate additive operation. The existing
Protocol 1.28 `dialog.open_folder` remains unchanged: its selected path can
never be supplied to `folder.read_entries` and never gains filesystem authority.

On selection, v2 returns exactly one of:

~~~json
{ "status": "selected", "path": "C:\\Example", "folderReference": "opaque-value" }
~~~

~~~json
{ "status": "cancelled" }
~~~

The `path` has the existing 32 KiB absolute-path bound and is display data
only. `folderReference` is exactly 22 unpadded base64url characters derived
from 128 bits of host randomness. It is opaque, session-bound, single-use, and
distinct from both selected-file and save references. Applications must not
parse, transform, persist, or share it.

`folder.read_entries` accepts exactly that reference and returns:

~~~json
{
  "status": "entries",
  "entries": [
    { "name": "notes", "kind": "directory" },
    { "name": "readme.txt", "kind": "file" }
  ],
  "complete": true
}
~~~

Each snapshot contains at most **32** direct child entries. An entry name is
one through 1,024 UTF-8 bytes, contains no NUL, ASCII control character, `/`,
or `\\`, and is never `.` or `..`. `kind` is exactly `file`, `directory`, or
`other`; a reparse point or any native type the host cannot safely classify is
`other`. Names only identify values in this response: they cannot be sent back
to the host or combined into a new authority-bearing path.

`complete` is false when the selected folder has more than 32 safe direct
entries. In that case `entries` is only a bounded native enumeration prefix;
the protocol has no cursor, page, continuation, or retry that exposes more.
The entry order is unspecified. A reference is consumed whether the snapshot
is complete, incomplete, or unavailable.

Unknown, expired, cross-session, malformed, already-consumed, identity-mismatch,
or native failures all map only to `folder.unavailable`. A malformed payload
uses the existing `request.payload_invalid` code. No result, error, event, or
diagnostic carries a native handle, canonical path, folder identity, child path,
entry count beyond the bounded response, timestamps, sizes, attributes, or
Windows status.

## Host-owned native identity

The UI thread may issue v2 only after its native adapter captures the exact
folder that Windows confirmed. On Windows, the adapter will open that folder
with direct APIs in directory mode, reject a selected reparse point, retain its
directory identity and a handle that denies replacement, rename, and deletion,
and create the reference only after all capture checks pass.

The later worker consumes that retained state. If it must reopen a private path
to enumerate, it must take a second protected directory handle and compare its
native identity to the captured one before reading any names. A changed,
missing, reparse, or mismatched target is unavailable rather than an
enumeration of a replacement. The protocol and portable layers never see the
handle, private path, or identity value.

At most 32 live folder references may exist in one session. Session shutdown
closes every retained folder object and revokes every remaining reference.

## Compatibility and deferred work

The portable core owns reference grammar, one-use lifetime, exact entry values,
and bounded result validation; it performs no filesystem I/O. A host without a
capture and enumeration adapter reports `folder.unavailable`.

Future work needs separate contracts and decisions: recursive traversal,
pagination, child selection, child-file reading, binary content, writing,
creation, deletion, rename, drag-and-drop, folder watches, persistent grants,
initial folder policy, multi-selection, non-Windows adapters, packaging, and
production identity.

## Planned verification

Portable tests will prove reference, entry, bound, one-use, and cross-session
rules. Windows tests will prove directory and reparse rejection, protected
identity comparison, bounded enumeration, cleanup, and no path-based fallback.
A development diagnostic will require selecting a folder, observing one direct
snapshot, and cancelling a second run; neither action may create, edit, or
retain a folder permission.
