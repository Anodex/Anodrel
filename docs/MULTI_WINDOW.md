# Anodrel session-owned multi-window contract

**Status:** The portable session-owned state and worker-to-UI creation handoff
are implemented. The direct Windows host can already create and route several
native windows, but its session-group integration and the Protocol 1.25 public
surface are still under implementation. No released protocol version yet lets
an application create one.

## Purpose

Applications need more than one surface for work such as a document and its
properties, a composition and its preview, or two independent documents. That
need must not become a general native-window bridge. A native handle, desktop
coordinate, monitor, global window list, or another process's window is not an
application resource.

The first public model is therefore a small set of **session-owned views**.
Each view has its own UI document, document revision, input queue, and opaque
logical identity. The host alone maps that logical identity to an operating
system window.

## Scope and limits

An authenticated UI session starts with one primary view whose logical identity
is `main`. It may have at most three open secondary views, for a maximum of
four open views in the session. The host creates a secondary view only on its
own UI thread and chooses its native style, initial size, position, monitor,
z-order, parentage, icon, and accessibility root.

`main` is an API spelling for the caller's already-associated primary view. It
is not a Win32 handle, a process ID, a pointer, or a cross-session name.
Secondary identities use the exact canonical spelling `window-<n>`, where `n`
is a nonzero base-10 integer without leading zeroes from 1 through 65535. The
host issues an identity only after it accepts a creation request. A closed
identity is never reused during that authenticated session.

An application can use only identities it received from its own session. There
is no enumerate, lookup, native-handle, geometry, monitor, visibility,
activation, focus-state, close-state, or lifecycle-event operation. A later
operation on a closed or unknown identity returns the existing safe
`window.unavailable` category; it does not reveal whether a person closed a
window or why the host no longer has it.

## Reserved Protocol 1.25 surface

Protocol 1.25 is reserved for the following exact operations. They are not
accepted by the current 1.24 host and must not be used by an application until
the implementation, compatibility tests, and host integration ship together.

| Operation | Exact payload | Exact success result | Required grants |
| --- | --- | --- | --- |
| `window.open` | `{ "title": string, "document": string }` | `{ "windowId": string }` | `window.open`, `ui.document.write` |
| `window.close` | `{ "windowId": string }` | `{ "status": "requested" }` | `window.close` |
| `ui.document.replace.window` | `{ "windowId": string, "document": string }` | `{ "revision": string }` | `ui.document.write` |
| `ui.events.read.window` | `{}` | bounded per-view semantic events | `ui.events.read` |

`window.open.title` follows the same 96 UTF-16-code-unit and no-control-
character rule as `window.title.set`. The host composes the visible caption
with the machine-validated application display name; it never assigns an
application string verbatim. `document` is one 24 KiB-or-smaller
`anodrel.ui.document.v1` encoded document. The new view begins at revision 1
only after the document validates completely. A rejected document or failed
native creation leaves no logical view behind.

The initial release intentionally accepts only document format v1. A future
scroll-capable view update needs a new exact operation rather than silently
making `ui.document.replace.window` accept v2 data.

`window.close` accepts only a currently issued secondary identity. It cannot
name `main`; `session.close` remains the one operation that asks the host to
end the entire authenticated session. Its success acknowledges that the host
accepted the close request, not that a native window was destroyed or that a
person saw anything.

`ui.events.read.window` returns no document data. Each accepted UI action is
tagged with the logical `windowId` that produced its revision-bound semantic
candidate. Each view retains at most 32 pending candidates and reports its own
overflow and stale/rejected counts. Cross-view events have no promised global
ordering: the application's model must not infer desktop timing from them.
The legacy targetless document and event operations retain their primary-view
meaning and cannot consume secondary-view traffic.

## Host lifetime

A host owns one session-window group. Its group owns the per-view native
mapping, cross-thread document mailboxes, input queues, and—where applicable—
the tracked verified child lifetime. Removing a secondary view drops only that
view's resources. Closing the final view ends the group. A granted
`session.close` request is group-wide: the host asks every view in that one
authenticated session to close before it releases the child and pipe workers.

No view grants authority over another application or session. Existing
targetless title, state, focus, fullscreen, size, menu, dialog, notification,
field, and file services keep their primary-view-only semantics until each has
its own separately reviewed multi-view operation. A secondary view initially
receives document rendering, local semantic input, and Windows accessibility
only; it does not inherit a generic native service bridge.

## Portable creation handoff

Before the host exposes Protocol 1.25, its portable UI-session group validates
the proposed initial document and reserves one secondary identity. It sends
the host-created context, revision-one snapshot, document mailbox, and input
mailbox through one take-once request for the owning UI thread. The caller
receives the logical identity only after that UI thread reports successful
native creation and registration.

At most one creation request is in flight for a group. It waits no more than
five seconds for the UI thread. A failed or timed-out request rolls its pending
logical view back; a late completion is rejected so the native host can destroy
the just-created window. This coordination contains no native handle or
application-chosen native setting, and it does not itself release a tracked
product child. Windows integration must retain that child until the final view
leaves.

## Security and compatibility rules

- `window.open` is distinct from `ui.document.write`: permission to render a
  document is not by itself permission to create operating-system windows.
- The protocol carries only opaque logical identities. All real handle lookup
  occurs on the UI thread after the host revalidates its own group mapping.
- The host creates, registers, and owns a view before showing its native
  window. If registration or native creation fails, it rolls back the pending
  logical view through the portable abort path before answering the pipe
  worker.
- A close request, timer wake-up, and native destroy path are idempotent. No
  completed or timed-out request can make an identity refer to a later window.
- Every secondary document has an independent monotonically increasing
  revision. A candidate from one view cannot be accepted against another
  view's same-numbered revision.
- A session may never have more than four open views or more than 65535 issued
  secondary identities. Both limits fail closed.

## Deferred work

This contract deliberately excludes application-selected bounds, positions,
monitors, native menus on secondary views, modal relationships, dialog owners,
window enumeration, event subscriptions, restoration, background execution,
and cross-session control. It also does not make a production packaging or
signed-launch decision.

See Decision 0092, `docs/WINDOW_LIFECYCLE.md`, and `docs/UI_SESSIONS.md`.
