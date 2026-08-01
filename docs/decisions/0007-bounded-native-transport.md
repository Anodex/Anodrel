# Decision 0007: Native transport starts with a bounded session engine

**Status:** Accepted

**Date:** 2026-07-31

## Context

Anodrel needs an application-to-host transport without inheriting a framework
IPC layer or letting stream parsing run unbounded in the Windows UI thread.
The public JSON protocol already defines requests, responses, capability policy,
and compatibility. A local byte stream still needs framing, strict resource
limits, and a well-defined failure policy.

## Decision

Anodrel owns a binary frame codec (`anodrel-wire`) and a host session engine
(`anodrel-transport`). Wire 1.0 uses an `ANDR` header, explicit version, UTF-8
payload length, and a JSON protocol payload. It accepts fragmented and
coalesced streams while limiting one payload to 64 KiB and one receive burst to
four complete frames.

The session engine owns a `CoreHost`, so only host-created policy reaches the
capability check. It converts complete inbound frames to protocol responses and
returns complete outbound frames. It does not listen on a port or pipe,
authenticate a caller, launch application content, or run work asynchronously.

The intended first OS adapter is a direct Windows named pipe. It is deliberately
deferred until application launch and session authentication are documented:
predictable pipe names or client-provided values must not become authority.

## Consequences

Positive:

- byte framing and protocol behavior are testable without OS handles or UI;
- memory and burst limits are fixed before untrusted content can connect;
- later adapters can move blocking I/O off the UI thread without duplicating
  parsing or policy behavior;
- applications remain independent from Windows pipe details.

Tradeoffs:

- the current engine is not an end-user IPC endpoint yet;
- an OS adapter, session bootstrap, cancellation scheduling, and back-pressure
  policy remain required before application traffic is accepted.

## Revisit conditions

Revisit only if an OS adapter proves that a byte-stream frame cannot meet a
required isolation or latency constraint. Any alternate runtime library still
requires the exception process in Decision 0005 and explicit user approval.
