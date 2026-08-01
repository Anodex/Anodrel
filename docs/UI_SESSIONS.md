# Anodrel UI session state v1

**Status:** Foundation contract. `anodrel-ui-session` owns in-memory document
replacement and revision-bound semantic-event validation. It has no transport,
native host, renderer, package, application identity, event queue, or operating-
system authority.

## Purpose and boundary

A future authenticated application needs a way to replace its current UI tree
without a renderer accepting ambiguous or stale data. This foundation defines
one state machine for a caller that has already been authenticated and assigned
to one host window. The caller—not this crate—owns session identity, connection
lifetime, update scheduling, rendering, event delivery, permission checks, and
shutdown.

The state machine consumes only the strict `anodrel.ui.document.v1` format from
`docs/UI_DOCUMENTS.md`. It never reads a file, package, pipe, socket, or native
window. A document and an action remain visual data only.

## Document replacement

A new `UiDocumentSession` starts empty at revision `0`. `replace_document`
accepts one encoded document and performs this sequence atomically:

1. enforce the interchange format's 64 KiB and exact-schema limits;
2. decode and validate the full bounded UI document; then
3. replace the current document and advance to the next nonzero revision.

If validation fails, the current document and revision are unchanged. A revision
never wraps: an exhausted revision space fails closed. `clear_document` removes
the current document and advances the revision only when a document existed, so
any later event from that document is stale.

The current state is exposed as the immutable document paired with its exact
revision. A renderer must associate a layout pass with that revision and
discard it after a replacement or clear.

## Semantic events

The host input adapter may submit one existing `UiEvent::ActionInvoked` together
with the revision used to produce its layout. The state machine returns a
`UiApplicationEvent` only when all of these are true:

- the supplied revision equals the current document revision;
- a current document exists; and
- its tree still contains that action ID as an enabled action.

The session cannot independently establish visibility or hit-test geometry; the
host must create the input `UiEvent` through the current document's layout. A
returned event contains only the revision and action ID. It does not invoke a
callback, send a protocol message, queue work, grant a capability, or execute a
native operation.

## Compatibility and future integration

This is a Rust state API, not an application wire operation. It has no client-
supplied session ID, request ID, event sequence, acknowledgement, or queue. A
future protocol or authenticated transport must define those fields, bounded
back-pressure, cancellation, error mapping, application identity binding, and
window lifecycle before it connects an external client to this state machine.

## Verification

The crate tests successful replacement, deterministic revisions, failed-update
state preservation, clear behavior, stale events, removed actions, disabled
actions, and semantic event identity. It depends only on Anodrel UI crates and
the Rust standard library.
