# Anodrel Protocol v1

**Status:** Foundation contract, version 1.4

This document defines the public, transport-neutral boundary between a Platform
application SDK and a host. It is intentionally limited to core operations
that do not expose operating-system authority. New platform services must be
documented here before their host implementation is added.

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
the host's. Version 1.4 accepts `{"major": 1, "minor": 0}` through
`{"major": 1, "minor": 4}`.

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

The current operations are:

| Operation | Payload | Result | Capability |
| --- | --- | --- | --- |
| `platform.ping` | `{ "sentAt": string }` | host receive time and host name | none |
| `platform.capabilities` | `{}` | application ID and current grants | none |
| `platform.health` | `{}` | ready status, host name, and version | `diagnostics.read` |
| `ui.document.replace` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.document.replace.v2` | `{ "document": string }` | accepted document revision | `ui.document.write` |
| `ui.events.read` | `{}` | bounded current UI events | `ui.events.read` |
| `session.close` | `{}` | accepted close request | `session.close` |

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

This operation takes up to **32** queued semantic input candidates from the
already authenticated session. It requires the host-issued `ui.events.read`
capability. Before returning a candidate, the host validates its document
revision and enabled action identity against the current session document. A
stale, removed, disabled, or missing action is never delivered.

The result is `{ "events": array, "dropped": number, "discarded": number }`.
`events` contains at most 32 typed event envelopes in input order. `dropped`
is the number of newer input candidates that could not enter the fixed 32-slot
host queue since the last read. `discarded` is the number taken from that queue
but rejected as stale or unavailable during validation. Both are nonnegative
safe integers. A caller that observes either nonzero value must treat its UI
state as potentially out of date and may replace the document again.

Version 1.2 defines the one event envelope below. It is carried in the read
result because Wire 1.0 has request/response framing; it is not an unsolicited
pipe write.

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
`request.invalid`, and `request.payload_invalid`. Error messages are suitable
for a developer log but must not contain secrets, raw paths, or native errors.

## Cancellation and events

A cancellation message contains `protocolVersion`, `kind: "cancel"`, and a
non-empty `cancellationId`. A host may cancel only work that has not completed;
it returns `request.cancelled` for a request that it observes before execution.
Later host operations will document operation-specific cancellation behavior.

Events are opt-in. Every event must include `protocolVersion`, `kind: "event"`,
`eventName`, `source`, `schemaVersion`, and a typed payload. Version 1.2
implements `ui.action.invoked` only through `ui.events.read`; it does not yet
provide subscriptions, unsolicited delivery, acknowledgements, or cancellation.

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
