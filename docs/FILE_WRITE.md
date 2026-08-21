# Selection-scoped file writing

**Status:** Implemented for the direct Windows UI-session host in Protocol 1.17.

Anodrel's legacy save picker is intentionally only a user choice. This document
defines the separate Protocol 1.17 text-write boundary. It does not make
`dialog.save_file` mutating.

## User-visible flow

~~~text
dialog.save_file.v2 -> user chooses one destination -> host captures one output object
file.write_text     -> application presents its opaque save reference and text once
~~~

`dialog.save_file.v2` requires the existing host-issued `dialog.save_file`
capability. Its successful result is:

~~~json
{ "status": "selected", "path": "C:\\Users\\Owner\\note.txt", "saveReference": "AbCdEfGhIjKlMnOpQrStUv" }
~~~

The path is display data only. `saveReference` is the authority, and it is a
host-created exact 22-character base64url value from 128 bits of CNG
randomness. It is not a path, file identifier, capability declaration, handle,
or durable permission. Applications must preserve it exactly, never parse,
persist, transform, or share it.

The only other result is `{ "status": "cancelled" }`. The legacy
`dialog.save_file` result remains `{ "status": "saved", "path": string }`
or cancellation and never contains a save reference.

## Write boundary

`file.write_text` requires a distinct host-issued `file.write_text` capability
and accepts exactly:

~~~json
{ "saveReference": "AbCdEfGhIjKlMnOpQrStUv", "text": "Hello, Anodrel." }
~~~

`text` is valid UTF-8 JSON text limited to **8 KiB of source bytes**. A
successful response is `{ "status": "written" }`. An unknown, consumed,
expired, cross-session, malformed, or otherwise unusable reference returns
only `file.unavailable`; text above the bound returns `file.text_too_large`.
No result or error contains native status, file identity, canonical path,
directory content, written text, byte count, or durability information.

No field may name a path, filename, directory, handle, encoding, offset,
length, append mode, overwrite mode, target window, or callback. There is no
readback, progress event, background transfer, or persistent grant.

## Windows capture and cleanup

Capture happens on the host UI thread immediately after the host-owned save
picker closes and before a pipe worker receives a result.

- An existing destination is opened once as a non-directory, non-reparse
  regular file with write access. The retained handle allows later readers but
  blocks new write, rename, and deletion access. Its current contents remain
  intact at capture time.
- When the selected filename did not exist, Windows creates it only through an
  exact create-new call. If another object appears first, capture fails rather
  than opening that replacement. The new file handle is marked for deletion
  while the reference is unused. Dropping that reference or ending its session
  deletes the empty object through the same handle; it never reopens the path
  to clean up.
- A successful write consumes the reference before mutating the handle. For a
  newly created destination it first cancels the pending deletion, then seeks
  to byte zero, writes all bounded bytes, truncates an old tail, and flushes.

This is an exact text replacement attempt, **not an atomic replacement**. If a
native call fails after writing begins, the destination may be empty, partial,
or partly replaced. Success means the bounded synchronous native sequence
reported success, not that data is crash-proof on every storage device.

## Security and lifecycle invariants

- A caller cannot turn a path from any dialog into write authority.
- `saveReference` and the read-side `selectionReference` are different types;
  neither operation accepts the other.
- Save selection and text writing need separate immediate grants.
- One reference is consumed exactly once, even when the write later fails.
- Session shutdown drops all handles and cleans up an unused newly created
  destination.
- The host does not log selected paths, written text, references, identities,
  handles, or native failures through the protocol.

## Deferred work

Atomic replacement, binary data, appending, partial or streaming writes,
durability reporting, file readback, multiple selections, folder access,
persistent permission, recovery, and non-Windows adapters are intentionally
outside this contract. Each needs a new decision and threat-model entry.

See Decision 0079, `docs/FILE_DIALOGS.md`, `docs/FILE_ACCESS.md`, and
`docs/PROTOCOL.md`.
