# Decision 0008: Windows transport uses an owned authenticated named pipe

**Status:** Accepted

**Date:** 2026-07-31

## Context

Decision 0007 defines a bounded Anodrel frame and policy-bound host session but
does not expose it to another process. The first Windows adapter must avoid a
framework IPC layer, a broad default named-pipe DACL, predictable endpoint names,
and client-supplied authority.

## Decision

The Windows adapter creates one direct duplex byte-mode named pipe using only
Win32 and CNG APIs. It generates 32 random bytes through `BCryptGenRandom`, uses
their hexadecimal representation as an opaque pipe-name suffix and session
token, and grants only the current Windows logon SID the individual pipe rights
required by a client. The descriptor includes required data, attribute, control,
and synchronization rights but excludes `FILE_CREATE_PIPE_INSTANCE`.

The first framed JSON message must be the private `session.authenticate` control
object. It contains a host-generated session ID and the 64-character token. The
transport session compares it without an early exit, acknowledges a successful
handshake, and only then dispatches public protocol requests through `CoreHost`.
Any framing or authentication failure ends the one-client connection.

The adapter offers synchronous `serve_one` specifically for a dedicated worker
thread. It must not execute on the Win32 UI thread. Decision 0009 defines the
separate private bootstrap adapter that can hand the invitation to the intended
child without putting the token in command lines, environment variables, logs,
or predictable files. Controlled application content remains a later boundary.

## Consequences

Positive:

- Anodrel owns endpoint creation, ACL construction, random credentials, framing,
  and dispatch rather than inheriting framework IPC semantics.
- A local client needs both an OS access check and the host-created secret;
  knowing the pipe name alone is insufficient.
- Stream I/O and UI lifecycle remain separable modules.

Tradeoffs:

- The current adapter proves one authenticated client only; multi-client,
  cancellation, concurrent work, and application launch remain later work.
- The pipe adapter delegates child process creation and bootstrap delivery to
  Decision 0009 rather than mixing process control into stream I/O.

## Revisit conditions

Revisit when a cross-platform adapter or application-launch contract proves that
the one-client model cannot meet isolation or performance requirements. Any
third-party runtime still requires explicit user approval under Decision 0005.
