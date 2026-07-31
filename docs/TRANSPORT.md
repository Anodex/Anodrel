# Anodrel Native Transport v1

**Status:** Foundation contract. The frame codec, authenticated host session,
and one-client Windows named-pipe adapter are implemented. Application launch
and content hosting are not implemented yet.

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
one client on a worker thread and receives an invitation that the host must hand
to a launched application through a future private bootstrap mechanism. A pipe
name alone is never authentication.

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
The host compares the token without an early exit.

The invitation is sensitive bootstrap material. The future application launcher
must not pass it through command-line arguments, environment variables, logs,
or a predictable on-disk location.

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

## Security boundary

The codec performs length validation before copying or decoding a payload. The
session passes only complete valid UTF-8 text to the core, which then performs
strict JSON and protocol validation. It intentionally does not expose an
application ID, session secret, operating-system handle, arbitrary command, or
direct native call.
