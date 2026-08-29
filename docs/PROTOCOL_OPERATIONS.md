# Anodrel Protocol v1 operation reference

## HTTPS text fetch

Protocol 1.19 implements `network.fetch_text` as one host-authorized HTTPS
`GET` request. The payload accepts exactly one bounded URL. The result contains
only `statusCode` (100 through 599) and at most 32 KiB of UTF-8 `text`; no
headers, redirects, address, certificate, timing, proxy, or native status is
observable. A host-selected exact-origin policy decides whether the service may
attempt the URL. Missing service, rejected origin, timeout, and native failure
all return `network.unavailable`; an oversized, non-UTF-8, or otherwise
unrepresentable response returns `network.response_invalid`. See
`docs/NETWORK.md` and Decision 0084.

## Session-window foreground request

Protocol 1.20 implements `window.focus.request` behind its separate
`window.focus` grant. It accepts exactly `{}` and returns exactly
`{ "status": "requested" }` when Windows accepts the owning host UI thread's
foreground request for that authenticated session's own window. The operation
cannot name a target, window handle, process, coordinate, monitor, input,
retry policy, callback, or accessibility element.

Windows may decline a foreground request under its own user-protection policy.
`window.unavailable` deliberately covers a missing session window, an expired
UI bridge, and such a refusal without revealing which occurred. The response
does not state whether the window became foreground, received keyboard focus,
moved in z-order, or was noticed by a person. A concurrent request returns
`window.busy`. See `docs/WINDOW_FOCUS.md` and Decision 0085.

## Session-window fullscreen request

Protocol 1.21 implements `window.fullscreen.set` behind its separate
`window.fullscreen` grant. It accepts exactly `{ "mode": "fullscreen" | "windowed" }`
and returns exactly `{ "status": "applied" }` when the owning host UI thread
accepts the requested reversible presentation action for that authenticated
session's own window.

The operation cannot name a target, window handle, process, monitor,
coordinate, size, style, display mode, z-order, visibility, keyboard shortcut,
callback, or accessibility element. On Windows, `fullscreen` means borderless
windowed fullscreen on the monitor Windows associates with that known window;
it is not exclusive display control. The host retains restoration facts
privately and `windowed` restores them. Duplicate requests for the current host
mode are accepted without revealing that mode.

`window.unavailable` covers a missing session window, expired bridge, and a
safe native-transition failure without revealing which occurred. A concurrent
request returns `window.busy`. The response never states resulting bounds,
monitor, style, visibility, or fullscreen state. See
`docs/WINDOW_FULLSCREEN.md` and Decision 0086.

## Session-window client-size request

Protocol 1.23 implements `window.size.set` behind its separate `window.size`
grant. It accepts exactly `{ "width": integer, "height": integer }`: width is
320 through 3840 and height is 240 through 2160, inclusive, in whole 96-DPI
logical client-area pixels. It returns exactly `{ "status": "applied" }` when
the owning host UI thread accepts the request for that authenticated session's
own window.

The operation cannot name a target, window handle, process, position, outer
bounds, monitor, DPI, presentation state, constraint, animation, callback, or
event. On Windows, the host derives the framed outer rectangle at the known
window's current DPI and preserves position, activation, and z-order. A
fullscreen session safely answers `window.unavailable` rather than changing
private restore facts. A concurrent request returns `window.busy`; neither
result reveals native geometry or current presentation. See
`docs/WINDOW_SIZE.md` and Decision 0088.

## Session-window presentation-state snapshot

Protocol 1.30 implements `window.state.get` behind the separate
`window.state.read` grant. It accepts exactly `{}` and returns exactly one
portable snapshot: `{ "state": "minimized" | "maximized" | "restored" }`.
The snapshot is read only from the authenticated session's own native window.

The operation cannot name a target, window handle, process, bounds, monitor,
fullscreen state, focus, visibility, z-order, timestamp, callback, event, or
subscription. It permits one in-flight read per session and reports
`window.busy` for another; an unavailable UI bridge or a read that does not
complete within five seconds reports `window.unavailable`. The response can
be stale as soon as it is returned. It never confirms a previous
`window.state.set` request took effect. See `docs/WINDOW_STATE_OBSERVATION.md`
and Decision 0117.

## Session-window coalesced presentation-state change

Protocol 1.31 implements `window.state.changes.read` behind the separate
`window.state.observe` grant. It accepts exactly `{}` and returns exactly one
field: `{ "state": "minimized" | "maximized" | "restored" | null }`.
A non-null value is the latest unread state transition seen for the
authenticated session's own native window; a newer transition replaces it.
`null` says only that no unread change is retained.

The first native observation establishes a baseline, so callers use
`window.state.get` when they need an initial state. This operation never waits
for a later change and cannot name a target, window handle, process, bounds,
monitor, fullscreen state, focus, visibility, z-order, timestamp, sequence,
count, history, callback, event, or subscription. An absent UI surface returns
`window.unavailable`. See `docs/WINDOW_STATE_CHANGES.md` and Decision 0118.

## Session-owned secondary views

Protocol 1.25 adds `window.open`, `window.close`,
`ui.document.replace.window`, and `ui.events.read.window`. `window.open`
requires both its separate `window.open` grant and the existing
`ui.document.write` grant. It accepts only a bounded single-line title proposal
and one 24 KiB-or-smaller `anodrel.ui.document.v1` document; the host composes
the displayed caption, creates the native view on its UI thread, and returns a
session-local opaque `windowId` only after successful registration. There can
be at most three open secondary views.

`window.close` needs its separate `window.close` grant and accepts only one
current canonical secondary `window-<n>` identity. It rejects `main`; the
existing `session.close` operation remains the group-wide path. Its success
means only that the host accepted the close request. A closed or unknown
identity returns `window.unavailable` without revealing native close state or
reason.

`ui.document.replace.window` accepts `main` or a current secondary identity and
updates only that view's strict v1 document and independent revision. The
targetless document operations remain primary-only. `ui.events.read.window`
accepts `{}` and returns only revision-checked semantic UI actions, each tagged
with its opaque `windowId`, plus aggregated bounded dropped and discarded
counts. Each of the at most four views retains 32 candidates, so one response
contains at most 128 tagged actions. It promises order only within an
individual view; it never exposes native handles, geometry, lifecycle events,
view enumeration, or desktop timing. See `docs/MULTI_WINDOW.md` and Decisions
0092–0093.

## Secondary scroll documents

Protocol 1.27 adds `window.open.v2` and
`ui.document.replace.window.v2`. Their payloads and successful window-ID or
revision results are identical to their version-1 counterparts, but each
requires an exact `anodrel.ui.document.v2` document. They retain the existing
`window.open` and `ui.document.write` grants. The host keeps each view's scroll
position and all local scrolling behavior; no position, listener, event,
callback, handle, or result is added. See `docs/SCROLLING.md`,
`docs/MULTI_WINDOW.md`, and Decision 0102.

## Semantic live-status documents

Protocol 1.26 adds `ui.document.replace.v3`, `window.open.v3`, and
`ui.document.replace.window.v3`. Their payloads and successful revision or
window-ID results are identical to their v1 counterparts, but each requires an
exact `anodrel.ui.document.v3` value. They retain the existing
`ui.document.write` and, for opening a view, `window.open` grants. No new
grant, callback, recipient, accessibility field, or result status is created.

Version 3 adds a visible semantic `status` node to the v2 document format. The
Windows host may publish one best-effort UI Automation live-region event after
a later changed visible status is applied to an established authenticated
session view. That delivery is intentionally not a protocol outcome: the
response acknowledges only document acceptance. See
`docs/UI_LIVE_ANNOUNCEMENTS.md` and Decision 0100.

## Native session menus

Protocol 1.18 implements the strict portable `menu.replace` boundary and its
separate `menu.write` grant. Protocol 1.24 optionally adds one canonical local
`shortcut` field to an item: `Ctrl+<key>` or `Ctrl+Shift+<key>`, with an
uppercase ASCII letter or digit as `<key>`. Version 1.18 through 1.23 requests
retain the original exact three-item-field grammar. No shortcut grants a new
capability or changes event shape; a current enabled action still arrives only
through the revision-checked `ui.events.read` route. A core with no attached
native menu service returns only `menu.unavailable`; the direct Windows
UI-thread bridge and interactive delivery are implemented. The development
diagnostic's remaining checks are a real desktop menu click and local shortcut.
`docs/MENUS.md` defines the exact model, menu-action event, and Windows
ownership rule.

## Diagnostic entries

Protocol 1.11 adds `diagnostics.entries.read`. It accepts exactly `{}` and
requires the existing immediate `diagnostics.read` host-issued grant. The result
is `{ "entries": [...] }`, with at most 64 records. Each record contains only
`sequence` (a canonical nonzero decimal string), `level` (`"info"`), and the
closed catalogue's fixed `component` and `event` strings. It accepts no filter,
cursor, time, source, path, free text, subscription, export, or acknowledgement.

A host that did not explicitly provide a bounded closed diagnostics source
returns `diagnostics.unavailable`, with no native error, path, request data, or
application data. The operation is a snapshot read: it does not persist, clear,
write, or subscribe to diagnostics.

## Credentials

Protocol 1.12 adds exact credential read, write, and delete operations. Their
`name` is 1 through 64 ASCII bytes of lowercase letters, digits, `.`, `-`, or
`_`, starting and ending with a lowercase letter or digit. Write's `secret` is
one non-empty canonical lowercase hexadecimal string of at most 4,096 bytes,
representing at most 2,048 opaque bytes. Read returns either
`{ "status": "found", "secret": string }` or `{ "status": "not_found" }`.
Delete returns `{ "status": "deleted" }` or `{ "status": "not_found" }`; write
returns `{ "status": "written" }`.

Each operation requires its own immediate host-issued grant. The boundary has
no target, application ID, metadata, enumeration, search, sharing, prompt,
subscription, export, acknowledgement, timestamp, or source field. Secret
values are never placed in diagnostics, events, logs, error text, or details.
Safe failures are `credential.unavailable`, `credential.access_denied`, and
`credential.stored_secret_invalid`.

## Storage state

Protocol 1.10 exposes exactly one host-derived application-state snapshot. It
does not expose a filesystem API. `storage.state.read` and
`storage.state.clear` accept exactly `{}`. `storage.state.replace` accepts
exactly `{ "snapshot": string }`, with a UTF-8 snapshot of at most **24 KiB**.
The host returns `request.payload_invalid` for any other payload or a larger
replacement before calling the storage service.

Each operation requires its matching immediate host-issued capability:
`storage.state.read`, `storage.state.replace`, or `storage.state.clear`.
Read returns `{ "status": "snapshot", "snapshot": string }` when a complete
saved snapshot exists, including an empty value, and `{ "status": "absent" }`
otherwise. Replace returns `{ "status": "replaced" }`; clear returns
`{ "status": "cleared" }`. The protocol has no key, path, filename, handle,
directory, range, stream, binary encoding, or temporary-name selector.

A storage service that cannot be safely called returns `storage.unavailable`.
Stored malformed UTF-8 and oversized state return `storage.snapshot_invalid`
and `storage.snapshot_too_large` respectively. Errors and diagnostics never
contain state data, paths, recovery source, temporary names, or native details.
Cancellation is honored only before the core begins the one bounded storage
operation; no operation continues in the background after its response.

## `dialog.open_file.v2` and `file.read_text`

Protocol 1.9 adds a selection-scoped text-read boundary. The new
`dialog.open_file.v2` operation keeps the exact bounded filter payload and
`dialog.open_file` capability from Protocol 1.7. Its successful result is
`{ "status": "selected", "path": string, "selectionReference": string }`;
the other result is `{ "status": "cancelled" }`. The returned path is still
display data only. `selectionReference` is the only value which can later
authorize a read; it is an opaque 22-character unpadded base64url value derived
from 128 bits of host randomness.

The host returns the selected result only after it has captured the selected
regular file as session-bound native read-only state. It returns only
`dialog.unavailable` if that capture or the picker cannot be serviced. The
legacy `dialog.open_file` result never contains a selection reference and
cannot be upgraded into one.

`file.read_text` requires the separate host-issued `file.read_text` capability
and accepts exactly `{ "selectionReference": string }`. No path, name, native
handle, directory, encoding, offset, or length field is valid. It consumes a
reference once and returns `{ "status": "text", "text": string }`, where the
strict UTF-8 source text contains at most **8 KiB**. The host returns
`file.unavailable` for an unknown, consumed, expired, cross-session, or native
unavailable reference; `file.text_too_large` for a larger file; and
`file.text_invalid` for non-UTF-8 content. None of those responses exposes a
path, metadata, native status, handle, or file text in diagnostics.

Both operations can return `request.cancelled` only when the host observes the
cancellation before work begins. A retained-file read performs one fixed,
bounded synchronous read once started; it does not retain a background transfer
after returning. Session shutdown revokes all outstanding references.

## `dialog.save_file.v2` and file output

Protocol 1.17 adds a separate retained-output-object boundary.
`dialog.save_file.v2` has the exact bounded filter payload and
`dialog.save_file` capability from Protocol 1.8. Its success result is
`{ "status": "selected", "path": string, "saveReference": string }`; its
other result is `{ "status": "cancelled" }`. A host can return a selected
result only after it captured a native output object for that exact user choice.
The existing `dialog.save_file` operation is unchanged: it remains a
non-mutating destination choice and can never be upgraded into a save
reference.

`file.write_text` needs the separate host-issued `file.write_text` capability
and accepts exactly `{ "saveReference": string, "text": string }`. Both
fields are required; no path, filename, directory, handle, encoding, offset,
length, append, overwrite, or atomicity field is valid. The 22-character
base64url save reference is opaque, session-bound, and consumed once. `text`
is limited to 8 KiB UTF-8 source bytes. Success is `{ "status": "written" }`;
an unavailable reference or native failure maps to `file.unavailable`, and an
oversized value maps to `file.text_too_large`.

Protocol 1.22 adds `file.write_binary` behind its distinct
`file.write_binary` capability. It accepts exactly `{ "saveReference": string,
"bytesBase64Url": string }`. `bytesBase64Url` is unpadded canonical base64url
for zero through **32 KiB** of decoded bytes: only the base64url alphabet is
valid, padding and whitespace are invalid, a remainder of one character is
invalid, and unused trailing bits must be zero. Invalid representation returns
`request.payload_invalid`; a canonical representation above the decoded bound
returns `file.binary_too_large`. Success is `{ "status": "written" }`.
After the capability check, the same one-use save reference is consumed even
when binary decoding fails; a text and binary operation cannot both consume it.
The operation accepts no path, name, type, handle, length claim, offset,
append, streaming, callback, or readback field.

The operation is synchronous and cancellation is observed only before it
starts. It makes one non-atomic replacement attempt through the retained native
object: a failure after mutation begins can leave partial content. Success is
not a durability or atomicity guarantee. `docs/FILE_WRITE.md`,
`docs/FILE_BINARY_WRITE.md`, and Decisions 0079 and 0087 define capture,
cleanup, and recovery limits.

## `dialog.save_file`

Protocol 1.8 adds the matching host-owned save picker. It uses the same strict
2 KiB filter payload and UI-thread boundary as `dialog.open_file`, but requires
the independent `dialog.save_file` capability. Its result is either a saved
destination (`{ "status": "saved", "path": string }`) or cancellation
(`{ "status": "cancelled" }`); selection never creates, truncates, or writes.
The destination is opaque application data, not write authority. The host
returns only `dialog.unavailable` when the picker cannot be serviced, without
a native error or destination path in diagnostics.

## `dialog.open_file`

Protocol 1.7 adds one host-owned open-file picker. It requires the host-issued
`dialog.open_file` capability and accepts exactly one `filters` array of one to
eight structured filters. Every filter has an ASCII non-control label of at
most 64 bytes and one to eight lowercase ASCII alphanumeric extensions of at
most 16 bytes. The full encoded payload is limited to **2 KiB**.

The result is either `{ "status": "selected", "path": string }` or
`{ "status": "cancelled" }`. A selected absolute path is opaque application
data, not filesystem authority; it does not allow reading, writing, directory
enumeration, process launch, or native-handle access. The native host displays
the modal picker on its own UI thread and may return only `dialog.unavailable`
when it cannot service the request. It exposes no raw Windows error, initial
directory, owner window, dialog flags, native handle, saved-history choice, or
path in a failure or diagnostic.

## `dialog.open_folder`

Protocol 1.28 adds one host-owned folder picker. It requires the independent
`dialog.open_folder` capability and accepts exactly `{}`. The result is either
`{ "status": "selected", "path": string }` or `{ "status": "cancelled" }`.
The selected path is absolute, non-empty, and at most **32 KiB** in UTF-8 bytes.

It is display data, not a retained folder permission: it does not permit
enumeration, reading, writing, creation, deletion, process launch, a handle,
or later folder operation. The request accepts no filter, initial folder,
title, owner window, multiple-selection flag, native dialog setting, callback,
or current-location readback. A host cannot service the picker only through
the safe `dialog.unavailable` category, without a path or native detail.

See `docs/FOLDER_DIALOGS.md` and Decision 0115.

## `dialog.open_folder.v2` and `folder.read_entries`

Protocol 1.29 adds a separate retained-folder route. `dialog.open_folder.v2`
requires the existing `dialog.open_folder` capability and accepts exactly
`{}`. Its result is `{ "status": "cancelled" }` or
`{ "status": "selected", "path": string, "folderReference": string }`.
The path has the same display-only rules as `dialog.open_folder`. The reference
is an opaque, exact 22-character unpadded base64url value derived by the host
from 128 bits of cryptographic randomness only after it retains a selected
non-reparse folder identity. It is session-bound, one-use, and distinct from
both file-read and file-write references. A session has at most 32 live folder
references, which are revoked when its host session closes.

`folder.read_entries` requires the separate `folder.read_entries` capability
and accepts exactly `{ "folderReference": string }`. It consumes a valid
reference before enumerating and returns only:

~~~json
{
  "status": "entries",
  "entries": [
    { "name": "notes.txt", "kind": "file" },
    { "name": "assets", "kind": "directory" }
  ],
  "complete": true
}
~~~

The response contains at most 32 direct entries. Every `name` is 1 through
1,024 UTF-8 bytes, is never `.` or `..`, and contains no control character,
`/`, or `\\`. `kind` is exactly `file`, `directory`, or `other`; a reparse
point or an entry that cannot be safely classified is `other`. `complete` is
false if more direct entries existed than fit in the response. Entry order is
unspecified. There is no cursor, page, continuation, recursive flag, child
path, child reference, content read, metadata, write, deletion, creation,
rename, watch, initial folder, title, native dialog setting, callback, or
native handle.

The host must reject a selected reparse folder and enumerate directly from the
retained directory handle; it must not reopen a path before it emits a name.
Any malformed, absent, expired, cross-session, consumed, or native failure
returns only `folder.unavailable`; it exposes no path, handle, raw
operating-system status, or partial result. See `docs/FOLDER_ACCESS.md` and
Decision 0116.

## `external.open`

Protocol 1.6 adds one external-link handoff operation. It requires the
host-issued `external.open` capability and accepts exactly `{ "url": string }`.
The UTF-8 URL is limited to **2 KiB** and must be one exact validated HTTPS
address defined by `docs/EXTERNAL_LINKS.md`. On success the result is
`{ "status": "opened" }`; it is only evidence that the operating system
accepted the handoff, not that a browser loaded a page or a user viewed it.

The operation never accepts an owner window, browser, handler, executable,
argument, working directory, scheme selector, file path, callback, or link
history selector. The host checks its capability immediately before invoking
the injected external-link service. A system handoff failure returns only
`external.unavailable`, with no native status, handler detail, or URL in the
error or diagnostics.

## `clipboard.read` and `clipboard.write`

Protocol 1.5 adds separate text-only clipboard operations. Both require an
already authenticated session and an immediate host-issued capability check.
`clipboard.read` accepts exactly `{}` and returns either
`{ "status": "text", "text": string }` or `{ "status": "no_text" }`.
The first form preserves the difference between an empty supported value and a
clipboard with no supported Unicode-text representation.

`clipboard.write` accepts exactly `{ "text": string }` and returns
`{ "status": "written" }`. The UTF-8 text field is limited to **24 KiB**,
leaving bounded space for its request envelope within Wire 1.0's 64 KiB frame.
The portable clipboard service may support a larger 64 KiB value only behind a
host boundary; this protocol operation never expands that transport-safe limit.

Neither operation accepts a clipboard owner, window, format, source,
application selector, history selector, or native handle. The host maps the
ordinary text format to its current operating-system clipboard. Native
contention, malformed system text, and an oversized system value return only
the stable `clipboard.unavailable`, `clipboard.text_invalid`, or
`clipboard.text_too_large` errors. Clipboard text itself never appears in
diagnostics or error text.

## `ui.document.replace`

This operation replaces the one current native UI document for the already
authenticated application session. `document` is one exact
`anodrel.ui.document.v1` JSON document as defined by `docs/UI_DOCUMENTS.md`.
It is data, not HTML, script, a window request, a callback, or a capability
declaration.

The encoded UTF-8 `document` string is limited to **24 KiB**. The stricter
limit sits below both the document format's 64 KiB maximum and Wire 1.0's 64
KiB message limit, leaving bounded space for the envelope and JSON escaping.
The host validates the whole document atomically through the strict document
codec. On failure it returns `request.payload_invalid`, exposes no raw document
content in diagnostics, and retains the prior document and revision.

On success, the result is `{ "revision": string }`. `revision` is a canonical,
nonzero base-10 unsigned integer string with no leading zero. It is opaque to
the application except for later event correlation; applications must preserve
it exactly rather than interpreting it as a JavaScript number. A replacement
always advances the revision. This operation has no incremental patch form,
window selection, document readback, action event, renderer attachment, or
native side effect.

## `ui.document.replace.v2`

Protocol 1.4 adds this explicit replacement operation for one exact
`anodrel.ui.document.v2` document. It has the same authenticated-session scope,
`ui.document.write` capability check, 24 KiB encoded limit, atomic revision
result, and safe failure behavior as `ui.document.replace`. It differs only in
the document codec: v2 may contain the bounded `scroll` node defined in
`docs/UI_DOCUMENTS.md`; its position remains host-owned runtime state and is
never part of the request or result. A v1 document passed to this operation is
invalid, and a v2 document passed to `ui.document.replace` is invalid.

## `ui.events.read`

This operation takes up to **32** queued semantic interaction candidates from
the already authenticated session. It requires the host-issued `ui.events.read`
capability. Before returning a document candidate, the host validates its
document revision and enabled action identity against the current session
document. Before returning a menu candidate, it validates the menu revision and
enabled semantic action against the current complete menu. A stale, removed,
disabled, or missing action is never delivered.

The result is `{ "events": array, "dropped": number, "discarded": number }`.
`events` contains at most 32 typed event envelopes in input order. `dropped`
is the number of newer input candidates that could not enter the fixed 32-slot
host queue since the last read. `discarded` is the number taken from that queue
but rejected as stale or unavailable during validation. Both are nonnegative
safe integers. A caller that observes either nonzero value must treat its UI
state as potentially out of date and may replace the document again.

Version 1.2 defines the document event envelope below. Protocol 1.18 also
defines `menu.action.invoked` with `source: "native.menu"`,
`schemaVersion: { "major": 1, "minor": 18 }`, and exactly
`{ "menuRevision": string, "action": string }` as its payload. Both are
carried in the read result because Wire 1.0 has request/response framing; they
are not unsolicited pipe writes.

~~~json
{
  "protocolVersion": { "major": 1, "minor": 2 },
  "kind": "event",
  "eventName": "ui.action.invoked",
  "source": "native.ui",
  "schemaVersion": { "major": 1, "minor": 0 },
  "payload": { "revision": "7", "action": "welcome.continue" }
}
~~~

`revision` is the exact canonical revision string returned by document
replacement. `action` is the document-unique enabled action element ID; it has
no command, callback, native operation, or capability meaning. Reading events
does not grant additional authority and never returns document content, paths,
credentials, or native diagnostics.

## `session.close`

This operation requests that the host end only the caller's already
authenticated session. It requires the host-issued `session.close` capability,
takes exactly `{}`, and returns `{ "status": "accepted" }` when the request
has entered the host's one-bit close signal. The response is not evidence that
a native window has been destroyed or that a process has exited.

The operation carries no window identity, title, geometry, process identifier,
reason string, callback, or target selector. The host alone decides which
resources belong to this session and performs any operating-system cleanup on
its own UI or lifecycle thread. A repeated request is harmless and remains
accepted. The close signal is a coalescing one-bit state, not a queue, event,
subscription, or general window-management API.

## Request example

~~~json
{
  "protocolVersion": { "major": 1, "minor": 0 },
  "kind": "request",
  "requestId": "b5e2b130-7a17-4aad-842d-1b1caa123456",
  "operation": "platform.health",
  "payload": {},
  "capabilityContext": {
    "applicationId": "com.example.app",
    "sessionId": "opaque-session-id",
    "grantedCapabilities": ["diagnostics.read"]
  }
}
~~~

