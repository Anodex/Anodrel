# Decision 0059: Stop Windows pipe workers through a host-only signal

**Status:** Accepted

**Date:** 2026-08-08

## Context

The first Windows pipe listener performs one synchronous accept and then
synchronous reads on a worker thread. A verified product session will own a
tracked child process and a native window. It must be able to release that
worker when either lifetime ends, including when the child fails before it can
authenticate. Application messages cannot safely control that operation.

## Decision

Each Windows pipe endpoint exposes a host-only stop signal. It stores no
invitation token, capability policy, or protocol data. A stop request sets a
shared host flag, makes one private local connection to wake an accept that has
not begun, and calls `CancelIoEx` for a pending accept or read.

The worker checks the flag before and after accepting and treats a
stop-requested cancelled I/O result as ordinary shutdown. It never sends a
protocol response or exposes native cancellation data.

## Consequences

The verified product-session coordinator can end a pipe worker when its child
or window shuts down, including when the child never reaches authentication.
The signal is unavailable to applications.

## Revisit conditions

Revisit when the listener gains multiple clients, asynchronous I/O, concurrent
request processing, or a cross-platform transport whose cancellation model
needs a common lifecycle abstraction.
