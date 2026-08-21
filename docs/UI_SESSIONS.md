# Anodrel UI session state v1

**Status:** Foundation contract. `anodrel-ui-session` owns in-memory document
replacement, revision-bound semantic-event validation, and a bounded input
mailbox. It has no transport, native host, renderer, package, application
identity, or operating-system authority.

## Purpose and boundary

A future authenticated application needs a way to replace its current UI tree
without a renderer accepting ambiguous or stale data. This foundation defines
one state machine for a caller that has already been authenticated and assigned
to one host window. The caller—not this crate—owns session identity, connection
lifetime, update scheduling, rendering, event delivery, permission checks, and
shutdown.

`replace_document` consumes only the strict `anodrel.ui.document.v1` format
from `docs/UI_DOCUMENTS.md`. The separate `replace_document_v2` method is an
explicit opt-in to `anodrel.ui.document.v2` scroll trees; it has the same atomic
revision behavior. Neither method reads a file, package, pipe, socket, or native
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

The state API has no client-supplied session ID, request ID, event sequence,
acknowledgement, or queue. `ui.document.replace` binds one authenticated
transport session to `replace_document` after its `ui.document.write`
capability check; `ui.document.replace.v2` separately binds that same session
to `replace_document_v2`. Each accepts only a 24 KiB encoded document and maps
validation failure to a safe protocol payload error. The state itself still has
no I/O or knowledge of either operation. Version 2 is explicit: a v1 document
cannot enter the v2 operation, and a scroll tree cannot enter the v1 operation.

The transport delivers semantic actions only through the bounded pull contract
below. It does not expose document readback, subscriptions, callbacks,
cancellation, or update back-pressure. Those need their own contracts before
this becomes a broader interactive application surface.

## Latest-document delivery

`UiDocumentMailbox` is a portable, per-session handoff for a host that must
move an already accepted document from a transport worker to another host
thread. It retains at most one immutable `UiDocumentSnapshot`: the latest
document and its revision. Publishing a newer snapshot replaces an older
pending snapshot; publishing an older revision has no effect. Taking a snapshot
clears that one pending value.

The mailbox has no I/O, timer, callback, application identity, renderer,
protocol event, or operating-system operation. It deliberately coalesces visual
updates rather than promising every intermediate frame. A host must create one
mailbox for one authenticated session and define how a native window is
notified or polls it. It must never use the mailbox for semantic action events.

The Windows UI Session Lab is the first consumer. It polls one explicitly
supplied mailbox on its UI thread and applies only a newer revision to its own
host-created view. The registered Windows-session adapter can now create the
same resources as one grouped host-owned interactive session, ready for a
future verified product window. It is not a public application window or a
launch path. See `docs/UI_SESSION_LAB.md` and Decision 0058.

## Semantic input delivery

`UiInputMailbox` is one shared per-session queue of at most 32 raw semantic
interaction candidates. A document producer can add only the document revision
and an `ActionInvoked` element ID that it derived from its own current layout.
On Windows, an enabled UI Automation button may offer that exact same candidate
through its bounded Invoke pattern (Decision 0069). A future native menu
producer can add only its current menu revision and a host-mapped semantic
action ID. The queue preserves insertion order across both kinds and has no
window target, native command identifier, data payload, callback, or operating
system call.

`ui.events.read` drains that queue through the authenticated transport. The
core revalidates document candidates through `UiDocumentSession::accept_event`
and menu candidates through `MenuSession::accept_action`. It returns a typed
`ui.action.invoked` or `menu.action.invoked` event only after the matching
current revision and enabled action are confirmed. The queue drops newer
candidates when full and records the count; a read also reports candidates
rejected during revision validation. This is a bounded pull delivery path, not
an event subscription, callback, or background queue.

## Verification

The crate tests successful replacement, deterministic revisions, failed-update
state preservation, clear behavior, stale events, removed actions, disabled
actions, semantic event identity, and ordered document/menu queue admission.
It depends only on Anodrel UI/menu crates and the Rust standard library.
