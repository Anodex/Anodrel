# Decision 0030: Native UI session state uses atomic document revisions

**Status:** Accepted

**Date:** 2026-08-01

## Context

The strict UI document codec defines individual valid trees, but a renderer and
an application also need to know whether an input event belongs to the current
tree. Without an explicit revision, a late event can be delivered after an
application replaces or removes the UI that created it. Making the renderer,
transport, or a global host object own that rule would couple one platform's
input timing to portable application state.

## Decision

Anodrel owns a small `anodrel-ui-session` crate. One caller creates one empty
session state, atomically replaces its document through the strict v1 decoder,
and receives a monotonically increasing revision. The crate exposes the current
immutable document with its revision and validates a supplied semantic action
only against that exact revision and a current enabled action.

The crate has no session identity, I/O, queue, renderer, raw input, window,
application package, callback, protocol request, or capability. It neither
establishes authentication nor delivers an accepted event to application code.
Those remain separate adapters with their own lifecycle and overload controls.

## Consequences

- a future renderer can discard stale layouts and a future transport can reject
  stale action events before they reach application logic;
- document replacement has explicit failure and clear behavior; and
- visual state remains independent from native authority and operating-system
  APIs.

## Revisit conditions

Revisit before adding incremental patches, multiple documents per session,
event queues, application event delivery, client-visible revisions, a protocol
operation, or native-window integration.
