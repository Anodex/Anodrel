# Selection-scoped binary file writing

**Status:** Implemented for the direct Windows UI-session host in Protocol
1.22. The first-party native binary-write template and real invited-pipe test
now cover the typed route; the interactive picker and created-byte check remain
a manual desktop verification.

Anodrel's existing text writer deliberately treats output as UTF-8 text. This
document defines the separate, bounded binary replacement boundary. It does
not broaden `file.write_text`, `dialog.save_file`, or a selected path into
general filesystem authority.

## User-visible flow

~~~text
dialog.save_file.v2 -> user chooses one destination -> host captures one output object
file.write_binary   -> application presents its opaque save reference and bytes once
~~~

`dialog.save_file.v2` keeps its existing `dialog.save_file` grant and result.
On a successful selection it returns an opaque `saveReference`; the display
path is not authority. A captured output object may be consumed once by either
the text or binary writer, never by both.

## Protocol boundary

Protocol **1.22** defines `file.write_binary`. It needs the distinct
host-issued `file.write_binary` grant and accepts exactly:

~~~json
{ "saveReference": "AbCdEfGhIjKlMnOpQrStUv", "bytesBase64Url": "AAEC_w" }
~~~

`saveReference` is the existing exact 22-character opaque reference. It is
session-bound, non-durable, unguessable, and consumed before the host mutates
the retained output object.

`bytesBase64Url` represents between zero and **32 KiB** of decoded bytes. It
is an unpadded, canonical base64url value:

- only ASCII `A-Z`, `a-z`, `0-9`, `-`, and `_` are allowed;
- `=` padding, whitespace, line breaks, and every other character are invalid;
- its length cannot leave one base64 character alone; and
- unused low bits in the final character must be zero, so one byte sequence has
  exactly one accepted representation.

Malformed, padded, non-canonical, or otherwise invalid values return
`request.payload_invalid`. A canonical value whose decoded result exceeds
32 KiB returns `file.binary_too_large`. An unknown, consumed, expired,
cross-session, or unusable save reference—or any native write failure—returns
only `file.unavailable`. Success is exactly:

~~~json
{ "status": "written" }
~~~

No result or error includes native status, file identity, canonical path,
directory contents, supplied bytes, decoded byte count, or durability detail.
The caller already knows the data it supplied; echoing it or measuring it adds
no useful authority.

No field may name a path, filename, directory, handle, MIME type, encoding,
offset, length, append mode, overwrite mode, target window, callback, or
transaction option. There is no readback, progress event, streaming upload,
background transfer, or persistent grant.

## Windows capture and cleanup

This operation reuses the exact retained `WindowsSaveFile` captured by
`dialog.save_file.v2`; it does not reopen the selected path. The direct Windows
adapter consumes the reference before changing the handle, cancels deletion for
an unused newly created destination, seeks to byte zero, writes all bounded
bytes, truncates an old tail, and flushes before reporting success.

This remains an exact synchronous replacement attempt, not an atomic replace.
A native failure after mutation begins may leave a new or existing destination
empty, partial, or partly replaced. Success means that bounded sequence
reported success, not that every storage device has made the bytes durable
against power loss.

The decoder is first-party pure code and runs in the portable protocol core;
the Windows adapter receives only already-bounded decoded bytes. No browser,
webview, Node.js runtime, codec library, path reopening, or application API
performs the write.

## Development verification

The Windows host includes a development-only UI-session diagnostic that opens
the host-controlled save picker, captures one output object, and writes the
fixed byte sequence `41 6E 6F 64 72 65 6C 00 FF` through its one-use reference:

~~~text
native\\target\\release\\anodrel-windows-host.exe --sample-ui-file-binary-write-client <node.exe> apps\\sample\\dist\\native-client.js
~~~

Choose a new temporary `.bin` filename. After the session closes, inspect the
file and confirm that exact sequence. Cancelling must leave no new file behind.
This is a diagnostic session path, not a product file permission or a general
filesystem bridge. This manual check is not yet recorded as passed.

The separate `init-file-binary-write` native template exercises the same path
without Node.js, with a fixed `.bin` filter and fixed bytes. See
`docs/NATIVE_FILE_BINARY_WRITE_TEMPLATE.md`.

## Security and lifecycle invariants

- A path from any dialog is never binary-write authority.
- Save selection, text writing, and binary writing retain separate immediate
  grants.
- A retained output object is consumed exactly once, even when decoding or the
  later native write fails.
- Session shutdown drops every remaining handle and removes an unused newly
  created destination through that handle.
- The host does not log selected paths, data, references, identities, handles,
  or native failure details through the protocol.

## Deferred work

Larger transfers, streaming, appending, partial writes, MIME handling, atomic
replacement, durability reporting, file readback, multiple selections, folder
access, persistent permission, recovery, and non-Windows adapters require
separate decisions.

See `docs/FILE_WRITE.md`, `docs/FILE_DIALOGS.md`, `docs/FILE_ACCESS.md`, and
Decision 0087.
