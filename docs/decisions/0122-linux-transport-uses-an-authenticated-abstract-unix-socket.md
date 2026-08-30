# Decision 0122: Linux transport uses an authenticated abstract Unix socket

**Status:** Accepted

**Date:** 2026-08-29

## Context

Anodrel's portable frame and authenticated-session engine are already separate
from Windows named-pipe details. The first Linux foundation needs the same
one-host, one-client local transport without opening a TCP port, leaving a
filesystem socket to clean up, accepting a client-selected endpoint, or adding
a framework IPC dependency.

A pathname Unix socket would require a directory, permissions, cleanup, and
replacement rules. Loopback TCP would enlarge the network boundary and provide
neither a per-user identity check nor a durable reason to avoid remote routing.
The existing Windows bootstrap record also names a Windows pipe and must not
be quietly reinterpreted as a cross-platform process-launch contract.

## Decision

The first Linux adapter is `anodrel-linux-pipe`. It creates exactly one
Linux-only `AF_UNIX` stream listener in the abstract namespace. Its private
name is `anodrel.v1.` followed by 64 lowercase hexadecimal characters produced
from 32 bytes read from Linux's `/dev/urandom` source. An abstract socket has
no filesystem entry, directory, pathname permission, cleanup race, or reusable
location.

The listener accepts one stream only. Before the stream reaches
`anodrel-transport`, the adapter obtains `SO_PEERCRED` directly from Linux and
requires the peer's effective UID to equal the host process's effective UID.
It also requires the existing first framed `session.authenticate` control with
a distinct 32-byte `/dev/urandom` token. The peer credential and token are
independent controls: a same-UID process that lacks the invitation cannot
authenticate, while another UID cannot use a leaked invitation.

The endpoint name, session ID, and token belong to a host-only invitation. Its
`Debug` form redacts the token and its destructor clears token storage. The
adapter provides no public client, TCP address, filesystem path, child launch,
bootstrap-delivery route, native window, capability, policy source, or service
adapter. A future Linux launcher must define its own child-only invitation
channel; it must not loosen the Windows `pipeName` bootstrap validation.

`serve_one` remains worker-thread-only. A host-only stop signal wakes a pending
accept through one private same-process connection; a connected worker uses a
short bounded read timeout to observe the stop request. Stop, failed peer
credential checks, malformed transport traffic, and I/O failures close the
one endpoint without adding a protocol response or native detail.

## Consequences

- Linux gains a real, directly tested local IPC foundation while the portable
  protocol, frame codec, and capability semantics remain unchanged.
- The adapter uses only Anodrel crates, the Rust standard library, and direct
  Linux interfaces (`AF_UNIX`, abstract names, `SO_PEERCRED`, `geteuid`, and
  `/dev/urandom`).
- A future Linux GUI host can compose the same authenticated session without
  importing Windows pipe code or making a Linux product-launch claim now.
- macOS is not included: it does not support Linux abstract socket names or
  `SO_PEERCRED` with the same contract.

## Alternatives considered

**A filesystem Unix socket.** It creates directory ownership, cleanup, stale
entry, and replacement policy before a host location contract exists. Refused.

**Loopback TCP.** It broadens the endpoint to a network abstraction and loses
the direct same-UID peer check. Refused.

**Token-only authentication.** A leaked invitation would become usable by any
local UID. Refused.

**Reuse the Windows bootstrap record.** Its validated pipe-name grammar is
intentionally Windows-specific. Refused.

## Revisit conditions

Revisit before adding a Linux child launcher, filesystem endpoint, TCP route,
multiple clients, cross-user connection, service adapter, GUI host, a
non-Linux Unix implementation, process identity, packaging, installation, or
updates. Each changes the authority or lifecycle boundary and needs its own
decision and verification.
