# Anodrel Protocol v1

**Status:** Implemented through version 1.27, including bounded semantic
live-status documents and exact scroll documents for session-owned secondary
views.

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
the host's. Version 1.27 accepts `{"major": 1, "minor": 0}` through
`{"major": 1, "minor": 27}`.

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
| `window.size.set` | `{ "width": integer, "height": integer }` | `{ "status": "applied" }` | `window.size` |
| `window.open` | `{ "title": string, "document": string }` | `{ "windowId": string }` | `window.open`, `ui.document.write` |
| `window.open.v2` | `{ "title": string, "document": string }` | `{ "windowId": string }` | `window.open`, `ui.document.write` |
| `window.open.v3` | `{ "title": string, "document": string }` | `{ "windowId": string }` | `window.open`, `ui.document.write` |
| `window.close` | `{ "windowId": string }` | `{ "status": "requested" }` | `window.close` |
| `menu.replace` | `{ "menus": [{ "label": string, "items": [{ "id": string, "label": string, "enabled": boolean, "shortcut"?: "Ctrl+<A-Z0-9>" \| "Ctrl+Shift+<A-Z0-9>" }] }] }` | current menu revision | `menu.write` |
| `ui.document.replace` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.document.replace.v2` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.document.replace.v3` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.document.replace.window` | `{ "windowId": string, "document": string }` | accepted view document revision | `ui.document.write` |
| `ui.document.replace.window.v2` | `{ "windowId": string, "document": string }` | accepted view document revision | `ui.document.write` |
| `ui.document.replace.window.v3` | `{ "windowId": string, "document": string }` | accepted view document revision | `ui.document.write` |
| `ui.events.read` | `{}` | bounded current UI events | `ui.events.read` |
| `ui.events.read.window` | `{}` | bounded per-view tagged UI events | `ui.events.read` |
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


## Operation reference

Every supported request and its bounded payload lives in the [Protocol operation reference](PROTOCOL_OPERATIONS.md).

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
Protocol 1.23 adds no error code; its client-size request reuses the same safe
`window.unavailable` and `window.busy` categories.
Protocol 1.24 adds no error code; malformed, duplicate, or premature menu
shortcuts reuse `request.payload_invalid`.
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
