# Anodrel Native Transport v1

**Status:** Foundation contract. The frame codec, authenticated host session,
one-client Windows named-pipe adapter, and private child-bootstrap adapter are
implemented. A separate no-script application package surface is
implemented in `docs/APPLICATIONS.md`; it does not use this transport.

## Purpose

This transport carries the JSON protocol over a local byte stream without
depending on a framework IPC layer. It is deliberately separate from both the
public protocol and any Windows named-pipe adapter so that framing, resource
limits, and capability policy can be tested in isolation.

~~~text
local byte stream -> anodrel-wire -> anodrel-transport -> anodrel-core
                         |                 |                  |
                    frame limits       session bounds      host policy
~~~

The first operating-system adapter is a direct Windows named pipe. It accepts
one client on a worker thread. A private bootstrap adapter can hand its
invitation to one launched child process. A pipe name alone is never
authentication.

## Frame format

All integer fields use unsigned little-endian encoding.

| Offset | Size | Field | Value |
| --- | ---: | --- | --- |
| 0 | 4 | Magic | ASCII `ANDR` |
| 4 | 2 | Wire major | `1` |
| 6 | 2 | Wire minor | `0` |
| 8 | 4 | Payload length | UTF-8 JSON byte count |
| 12 | n | Payload | One documented Anodrel JSON envelope |

The wire version governs only this byte framing. The JSON payload retains its
own `protocolVersion`, so either layer can evolve without silently changing the
other.

## Resource and failure rules

- A payload is at most **65,536 bytes** before decoding.
- A decoder retains at most **four complete frames** (262,192 bytes including
  headers) from one receive operation.
- The decoder accepts fragmented and coalesced byte-stream input.
- Bad magic, an unsupported wire version, a declared oversize payload, invalid
  UTF-8, or more than four immediately queued frames ends the session. The
  future OS adapter must close its stream rather than trying to resynchronize.
- Frames are handled synchronously in arrival order by the session engine. A
  future adapter must move blocking stream I/O and long-running work away from
  the Windows UI thread.
- The frame contains no identity or capability grant. The host creates policy
  before it constructs the session; client-supplied context remains untrusted.

## Compatibility

Wire 1.0 accepts only wire major 1, minor 0. Additive compatible frame metadata
requires a new minor version and decoder test. A breaking framing change
requires a new major version. JSON protocol compatibility remains governed by
`docs/PROTOCOL.md`.

## Session authentication

The first complete frame on a new pipe must be this exact control object before
any public protocol envelope:

~~~json
{
  "kind": "session.authenticate",
  "sessionId": "host-generated opaque ID",
  "token": "64 lowercase hexadecimal characters"
}
~~~

`sessionId` is non-empty and at most 128 UTF-8 bytes. `token` represents 32
bytes produced by Windows CNG. On success the host sends:

~~~json
{ "kind": "session.authenticated" }
~~~

The session then accepts only documented Anodrel protocol requests. A malformed
or failed handshake, a public request before authentication, or a second
authentication attempt ends the connection without a public-protocol response.
The host compares the token without an early exit. For a registered
application, the host must create its policy before the pipe session by mapping
only the already validated machine record through `anodrel-session-policy`.
Neither this handshake nor its bootstrap invitation carries an application ID
or capability grant.

The invitation is sensitive bootstrap material. It must not pass through
command-line arguments, environment variables, logs, or a predictable on-disk
location.

## Windows named-pipe adapter

`anodrel-windows-pipe` owns the Windows-only listener. It generates a 32-byte
random suffix with `BCryptGenRandom`, creates `\\.\pipe\anodrel.v1.<suffix>`,
and accepts exactly one duplex byte-stream client. Its SDDL security descriptor
grants only the current Windows logon SID the pipe data, attribute, control, and
synchronization rights required for a client; it deliberately excludes
`FILE_CREATE_PIPE_INSTANCE` and does not use a broad default DACL.

The adapter uses a 4 KiB fixed read buffer and passes chunks to the bounded
session engine. It performs blocking pipe I/O only in the caller's worker
thread; the Windows message loop must never call `serve_one` directly.

### Startup Lab loopback

The Windows Startup Lab uses a temporary private loopback only as a native
transport smoke test. Before it creates its diagnostic window, the host creates
one ordinary current-session pipe with a CNG-generated invitation, connects an
internal in-process client, and runs the server on a worker thread. The client
sends the invitation-derived `session.authenticate` frame, then one
`platform.health` request, and the host waits for both valid responses before
the visual card can report ready. The invitation is never rendered or logged
and is dropped after the self-test.

This is not an application transport endpoint: it does not start a public
client, launch an executable, grant a privileged capability, or establish
executable identity. It exists to exercise the same DACL, authentication,
frame, decode, and policy path that a future controlled application connection
will use.

## Private child bootstrap

`anodrel-windows-bootstrap` delivers one pipe invitation to a child process
over an inherited anonymous standard-input handle. It does not expose that
handle to the protocol surface and it is not a second request channel.

The host creates the anonymous pipe, marks only the child read endpoint as
inheritable, and passes an explicit three-handle inheritance list to
`CreateProcessW`: bootstrap standard input and `NUL` standard output/error.
The write endpoint never becomes inheritable. The host writes exactly one
bootstrap frame and closes its endpoint. End-of-file is part of the contract:
the child must reject a truncated frame and must not wait for a second message.

The bootstrap frame is intentionally distinct from the `ANDR` application
transport frame:

~~~text
0                   4                   8                  12
+-------------------+-------------------+-------------------+
| magic: "ANBI"      | major: u16 LE     | minor: u16 LE     |
+-------------------+-------------------+-------------------+
| payload length: u32 LE                                  |
+----------------------------------------------------------+
| UTF-8 JSON payload (at most 2,048 bytes)                 |
+----------------------------------------------------------+
~~~

Version `1.0` payload fields are exact; unknown, missing, duplicate, or
wrongly typed fields are rejected:

~~~json
{
  "kind": "bootstrap.invitation",
  "pipeName": "\\\\.\\pipe\\anodrel.v1.<random suffix>",
  "protocolVersion": { "major": 1, "minor": 0 },
  "sessionId": "host-created session ID",
  "token": "64 lowercase hexadecimal characters"
}
~~~

The payload is secret material. It may be read only from the child standard
input, used to authenticate the first named-pipe frame, and then discarded.
It must never be echoed to stdout/stderr, a log, telemetry, crash reporting,
or a durable file. The current launcher has no application trust policy,
restart manager, or privilege boundary beyond the selected child executable.
The host now has a separate no-script text package surface under Decision 0010,
but that surface is not connected to this launcher or pipe. Publisher trust,
verified executable launch, and a capability bridge remain required before a
product application is launched by the Windows host.

## Development sample path

The repository includes a Windows-only development probe that proves the full
private path: the direct host creates a real pipe session, launches the sample
with an `ANBI` invitation, and the sample authenticates before issuing one
`platform.health` request. Its client adapter uses only Node.js built-in stream
and named-pipe APIs; it adds no shipped native runtime dependency.

This is a protocol and lifecycle demonstration, not controlled application
content hosting. The sample executable is supplied explicitly by the developer,
the launcher does not verify its identity, and the host exits after the probe.
It must not be used to launch a product application or treated as a webview,
renderer, package verifier, update mechanism, or application sandbox.

## Security boundary

The codec performs length validation before copying or decoding a payload. The
session passes only complete valid UTF-8 text to the core, which then performs
strict JSON and protocol validation. It intentionally does not expose an
application ID, session secret, operating-system handle, arbitrary command, or
direct native call.
