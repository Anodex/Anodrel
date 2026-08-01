# Decision 0054: Native cancellation is bounded, ordered, and pre-execution

**Status:** Accepted

**Date:** 2026-08-01

## Context

The public protocol, SDK, and mock host already define a `cancel` control with
an opaque cancellation ID. The authenticated native transport previously
forwarded every post-authentication frame to the core as though it were a
request. That left the real Windows pipe unable to honour the documented
pre-execution cancellation rule and made cancellation-only traffic an
unbounded future-state risk.

## Decision

The authenticated native session accepts `cancel` only after authentication.
It validates the existing protocol version and bounded cancellation identity,
stores at most 32 distinct unresolved IDs, and sends no response for a valid
control. When a later valid request carries one stored ID, the transport removes
that ID and returns `request.cancelled` without invoking the core operation.

Processing remains synchronous and ordered. A cancellation received after its
request has completed cannot roll it back. Invalid or unsupported cancellation
controls, and a new 33rd unresolved ID, close the session. Duplicate pending
controls do not consume additional capacity.

## Consequences

Positive:

- the SDK, mock host, and real native pipe now share the same useful
  pre-execution cancellation meaning;
- cancellation never creates a worker, callback, rollback path, or hidden
  ambient authority;
- cancellation-only input has a fixed memory ceiling.

Tradeoffs:

- synchronous operations cannot be interrupted once they have begun;
- a client that sends cancellations without matching requests can exhaust its
  own session after 32 distinct IDs;
- long-running or streaming operations need separate explicit cancellation and
  cleanup contracts before they are introduced.

## Revisit conditions

Revisit before adding concurrent request execution, streaming operations,
long-running cancellable work, multiple transport consumers, or any operation
that needs compensating cleanup after it begins.
