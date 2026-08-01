# Decision 0035: UI actions use bounded authenticated pull delivery

**Status:** Accepted

**Date:** 2026-08-01

## Context

The Session Lab can render an authenticated document, but a visible action is
not useful until the application can learn about it. Letting a UI callback call
application code directly would cross the Windows UI and pipe-worker boundary,
and an unbounded event channel would let rapid input consume memory. Input also
must be checked again after document replacement so a late click cannot target
an earlier tree.

## Decision

One session owns a 32-slot `UiInputMailbox`. Its native view may add only raw
`ActionInvoked` candidates with the revision used to lay them out. The
authenticated transport exposes `ui.events.read` in Protocol 1.2, gated by the
new `ui.events.read` policy capability. The core drains candidates in order and
uses its current `UiDocumentSession` to accept only current enabled actions.

The response wraps accepted data as `ui.action.invoked` event envelopes. It
also reports queue overflow and validation discard counts. The protocol uses a
client pull because Wire 1.0 is request/response; no background pipe write,
callback, subscription, acknowledgement, or cancellation is introduced.

## Consequences

- semantic input reaches an authenticated application session without native
  authority;
- late input is fail-closed by revision and enabled-action checks;
- pending input memory is bounded and loss is visible to the application; and
- subscriptions and event back-pressure policies remain future work.

## Revisit conditions

Revisit before adding event payload data, pointer coordinates, text entry,
subscriptions, background event writes, multiple UI consumers, or any native
effect for an action.
