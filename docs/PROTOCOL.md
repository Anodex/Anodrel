# Anodrel Protocol v1

**Status:** Implemented through version 1.22, including separately granted,
bounded binary file output documented in `docs/FILE_BINARY_WRITE.md`.

This document defines the public, transport-neutral boundary between a Platform
application SDK and a host. Its operations are deliberately bounded and carry
only explicit, host-issued operating-system authority. New platform services
must be documented here before their host implementation is added.

## Boundary and trust

An application uses the SDK to create an **unbound request**. A transport
adapter authenticates the application session and attaches its host-derived
capability context before a native host receives the request. Rendered content
cannot create, change, or elevate that context.

~~~text
Application SDK -- unbound request --> transport adapter -- host-bound request --> host
                                             |                       |
                                             +-- authenticated session +-- capability context
~~~

All messages must be JSON-compatible objects. Transport details, including a
future webview bridge, local socket, or in-process mock transport, are not part
of this protocol.

## Version compatibility

`protocolVersion` is an object with numeric `major` and `minor` fields. A host
accepts requests with its own major version and a minor version no greater than
the host's. Version 1.22 accepts `{"major": 1, "minor": 0}` through
`{"major": 1, "minor": 22}`.

- Additive fields and operations increase the minor version. Receivers ignore
  unknown additive object fields.
- Removing or changing the meaning of a field, error code, or operation
  increases the major version.
- Hosts return `protocol.version_unsupported` when they cannot safely process a
  request. Applications must not retry that request unchanged.

## Request envelope

The SDK constructs the following fields. The transport adds `capabilityContext`
only after binding an authenticated application session.

| Field | Meaning |
| --- | --- |
| `protocolVersion` | Requested protocol version. |
| `kind` | Always `request`. |
| `requestId` | Non-empty caller-generated opaque ID, unique within a session and at most 256 UTF-8 bytes. |
| `operation` | Documented operation name, at most 128 UTF-8 bytes. |
| `payload` | Typed operation input. |
| `cancellationId` | Optional opaque identity used by a separate cancellation message, at most 256 UTF-8 bytes. |
| `capabilityContext` | Host-issued application ID, session ID, and granted capabilities. |

The implemented operations are:

| Operation | Payload | Result | Capability |
| --- | --- | --- | --- |
| `platform.ping` | `{ "sentAt": string }` | host receive time and host name | none |
| `platform.capabilities` | `{}` | application ID and current grants | none |
| `platform.health` | `{}` | ready status, host name, and version | `diagnostics.read` |
| `diagnostics.entries.read` | `{}` | bounded closed host diagnostic records | `diagnostics.read` |
| `credential.read` | `{ "name": string }` | exact secret or not found | `credential.read` |
| `credential.write` | `{ "name": string, "secret": string }` | written | `credential.write` |
| `credential.delete` | `{ "name": string }` | deleted or not found | `credential.delete` |
| `notification.show` | `{ "title": string, "body": string }` | `{ "status": "shown" }` | `notification.show` |
| `window.title.set` | `{ "title": string }` | `{ "status": "applied" }` | `window.title` |
| `ui.fields.read` | `{}` | whole-surface current values | `ui.fields.read` |
| `window.state.set` | `{ "state": "minimized" \| "maximized" \| "restored" }` | `{ "status": "applied" }` | `window.state` |
| `window.focus.request` | `{}` | `{ "status": "requested" }` | `window.focus` |
| `window.fullscreen.set` | `{ "mode": "fullscreen" \| "windowed" }` | `{ "status": "applied" }` | `window.fullscreen` |
| `menu.replace` | `{ "menus": [{ "label": string, "items": [{ "id": string, "label": string, "enabled": boolean }] }] }` | current menu revision | `menu.write` |
| `ui.document.replace` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.document.replace.v2` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.events.read` | `{}` | bounded current UI events | `ui.events.read` |
| `session.close` | `{}` | accepted close request | `session.close` |
| `clipboard.read` | `{}` | bounded Unicode text or no text | `clipboard.read` |
| `clipboard.write` | `{ "text": string }` | accepted write | `clipboard.write` |
| `external.open` | `{ "url": string }` | accepted operating-system handoff | `external.open` |
| `network.fetch_text` | `{ "url": string }` | bounded UTF-8 text plus HTTP status | `network.fetch` |
| `dialog.open_file` | `{ "filters": [{ "label": string, "extensions": [string] }] }` | selected path or cancellation | `dialog.open_file` |
| `dialog.save_file` | `{ "filters": [{ "label": string, "extensions": [string] }] }` | save destination or cancellation | `dialog.save_file` |
| `dialog.open_file.v2` | `{ "filters": [{ "label": string, "extensions": [string] }] }` | selected path plus selection reference, or cancellation | `dialog.open_file` |
| `file.read_text` | `{ "selectionReference": string }` | bounded UTF-8 text | `file.read_text` |
| `dialog.save_file.v2` | `{ "filters": [{ "label": string, "extensions": [string] }] }` | selected path plus save reference, or cancellation | `dialog.save_file` |
| `file.write_text` | `{ "saveReference": string, "text": string }` | accepted bounded text replacement | `file.write_text` |
| `file.write_binary` | `{ "saveReference": string, "bytesBase64Url": string }` | accepted bounded binary replacement | `file.write_binary` |
| `storage.state.read` | `{}` | bounded saved snapshot or absence | `storage.state.read` |
| `storage.state.replace` | `{ "snapshot": string }` | accepted replacement | `storage.state.replace` |
| `storage.state.clear` | `{}` | accepted clear | `storage.state.clear` |

### HTTPS text fetch

Protocol 1.19 implements `network.fetch_text` as one host-authorized HTTPS
`GET` request. The payload accepts exactly one bounded URL. The result contains
only `statusCode` (100 through 599) and at most 32 KiB of UTF-8 `text`; no
headers, redirects, address, certificate, timing, proxy, or native status is
observable. A host-selected exact-origin policy decides whether the service may
attempt the URL. Missing service, rejected origin, timeout, and native failure
all return `network.unavailable`; an oversized, non-UTF-8, or otherwise
unrepresentable response returns `network.response_invalid`. See
`docs/NETWORK.md` and Decision 0084.

### Session-window foreground request

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

### Session-window fullscreen request

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

### Native session menus

Protocol 1.18 implements the strict portable `menu.replace` boundary and its
separate `menu.write` grant. A core with no attached native menu service returns
only `menu.unavailable`; the direct Windows UI-thread bridge and interactive
delivery are implemented. The development template's remaining check is a real
desktop menu click. `docs/MENUS.md` defines the exact model, menu-action event,
and Windows ownership rule.

### Diagnostic entries

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

### Credentials

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

### Storage state

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

### `dialog.open_file.v2` and `file.read_text`

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

### `dialog.save_file.v2` and file output

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

### `dialog.save_file`

Protocol 1.8 adds the matching host-owned save picker. It uses the same strict
2 KiB filter payload and UI-thread boundary as `dialog.open_file`, but requires
the independent `dialog.save_file` capability. Its result is either a saved
destination (`{ "status": "saved", "path": string }`) or cancellation
(`{ "status": "cancelled" }`); selection never creates, truncates, or writes.
The destination is opaque application data, not write authority. The host
returns only `dialog.unavailable` when the picker cannot be serviced, without
a native error or destination path in diagnostics.

### `dialog.open_file`

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

### `external.open`

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

### `clipboard.read` and `clipboard.write`

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

### `ui.document.replace`

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

### `ui.document.replace.v2`

Protocol 1.4 adds this explicit replacement operation for one exact
`anodrel.ui.document.v2` document. It has the same authenticated-session scope,
`ui.document.write` capability check, 24 KiB encoded limit, atomic revision
result, and safe failure behavior as `ui.document.replace`. It differs only in
the document codec: v2 may contain the bounded `scroll` node defined in
`docs/UI_DOCUMENTS.md`; its position remains host-owned runtime state and is
never part of the request or result. A v1 document passed to this operation is
invalid, and a v2 document passed to `ui.document.replace` is invalid.

### `ui.events.read`

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

### `session.close`

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

### Request example

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

## Responses and errors

Each response repeats `requestId`, sets `kind` to `response`, and includes safe
diagnostics containing only `hostName`. Responses have one of two forms:

~~~json
{
  "protocolVersion": { "major": 1, "minor": 0 },
  "kind": "response",
  "requestId": "b5e2b130-7a17-4aad-842d-1b1caa123456",
  "status": "success",
  "result": { "status": "ready", "hostName": "anodrel-host", "protocolVersion": { "major": 1, "minor": 0 } },
  "diagnostics": { "hostName": "anodrel-host" }
}
~~~

~~~json
{
  "protocolVersion": { "major": 1, "minor": 0 },
  "kind": "response",
  "requestId": "b5e2b130-7a17-4aad-842d-1b1caa123456",
  "status": "failure",
  "error": {
    "code": "capability.denied",
    "message": "platform.health requires the diagnostics.read capability.",
    "retryable": false
  },
  "diagnostics": { "hostName": "anodrel-host" }
}
~~~

Version 1 defines these stable error codes: `capability.denied`,
`operation.unsupported`, `protocol.version_unsupported`, `request.cancelled`,
`request.invalid`, `request.payload_invalid`, `clipboard.unavailable`,
`clipboard.text_invalid`, and `clipboard.text_too_large`. Error messages are
suitable for a developer log but must not contain secrets, raw paths, native
errors, clipboard text, or external-link URLs. Protocol 1.6 adds
`external.unavailable`.
Protocol 1.7 adds `dialog.unavailable`.
Protocol 1.9 adds `file.unavailable`, `file.text_invalid`, and
`file.text_too_large`.
Protocol 1.17 reuses `file.unavailable` and `file.text_too_large` for the
separate retained-output-object text-write boundary; it adds no new error code.
Protocol 1.18 adds `menu.unavailable` for a host that cannot attach or update
its own native session menu.
Protocol 1.19 adds `network.unavailable` and `network.response_invalid` for
its HTTPS text-fetch boundary.
Protocol 1.20 adds no error code; its window-focus request reuses the existing
safe `window.unavailable` and `window.busy` categories.
Protocol 1.21 adds no error code; its fullscreen request reuses the same safe
`window.unavailable` and `window.busy` categories.
Protocol 1.22 adds `file.binary_too_large`; malformed binary encodings reuse
the existing `request.payload_invalid` category.
Protocol 1.10 adds `storage.unavailable`, `storage.snapshot_invalid`, and
`storage.snapshot_too_large`.

## Cancellation and events

A cancellation message contains `protocolVersion`, `kind: "cancel"`, and a
non-empty `cancellationId`. It has no response. A host may cancel only work
that has not started; it returns `request.cancelled` for a request that it
observes before execution. Later host operations will document operation-specific
cancellation behavior.

Events are opt-in. Every event must include `protocolVersion`, `kind: "event"`,
`eventName`, `source`, `schemaVersion`, and a typed payload. Version 1.2
implements `ui.action.invoked` only through `ui.events.read`; it does not yet
provide subscriptions, unsolicited delivery, acknowledgements, or cancellation.
Protocol 1.18 reserves `menu.action.invoked` in the same bounded pull result;
the direct Windows adapter publishes it only after the common interaction
mailbox accepts its current command. It carries only a host-validated menu
revision and semantic action ID.

## Security rules

- The host, not application content, is authoritative for identity and grants.
- A capability check happens immediately before the operation is executed.
- Requests and responses are validated at the host boundary.
- Protocol diagnostics are safe metadata only; detailed native failures stay in
  host-controlled logs.
- Filesystem, process, credential, and other privileged operations require a
  threat model and an operation-specific contract before implementation.

## Modularity and performance

The protocol package contains plain data types and validation only. SDKs own
request lifecycle behavior, while hosts own policy and operating-system work.
The mock host is structurally compatible with the SDK transport but does not
depend on the SDK module, which keeps the dependency graph one-way.

The protocol framing contract is documented in `docs/TRANSPORT.md`.
Wire 1.0 limits one UTF-8 JSON payload to 64 KiB and one receive burst to four
complete frames before protocol parsing. The future OS adapter must additionally
set and test authenticated-session queue, timeout, and cancellation limits
before it accepts untrusted rendered content.

The direct Windows core and wire engine reject encoded messages larger than 64
KiB before JSON parsing. The codec rejects duplicate keys, malformed Unicode,
trailing data, and nesting beyond 64 levels. An authenticated transport can
publish a successful UI document replacement into a bounded per-session
mailbox. The development-only Windows UI Session Lab consumes one supplied
mailbox in one host-controlled native view; it is not a public application
window. Its separate 32-candidate semantic-input mailbox is exposed only
through the capability-gated `ui.events.read` pull operation. The host still
has no subscriptions, asynchronous privileged operations, or production event
queue; those requirements remain part of the host acceptance gate.

## Compatibility tests

`tests/contract/src/protocol-contract.test.ts` is the initial compatibility
suite. It verifies successful messages, capability enforcement, version
rejection, validation failures, cancellation before execution, and SDK error
mapping against the mock host. The native core has matching Rust unit
tests for its implemented request paths; a future SDK-native transport must run
the shared contract suite before it can expose these operations to an app.
