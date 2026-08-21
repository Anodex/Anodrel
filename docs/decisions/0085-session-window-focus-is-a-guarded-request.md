# Decision 0085: Session window focus is a guarded request

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel's session window can already receive a bounded title proposal and a
closed presentation-state command. A desktop application also needs to ask for
attention after a user-initiated workflow returns to its own surface. Calling
that a general "window API" would hide a major distinction: an application
bringing its own host window forward is not authority to name another window,
inspect the foreground process, inject input, or override Windows' foreground
protection.

Windows provides `SetForegroundWindow` for a desktop process, but Windows may
refuse the request so another application cannot interrupt a person who is
working elsewhere. Its return value does not justify a protocol state-read
surface. See Microsoft's [SetForegroundWindow documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow).

## Decision

Protocol 1.20 adds the separately granted `window.focus` capability and one
exact `window.focus.request` operation. Its payload is exactly `{}` and its
success result is exactly `{ "status": "requested" }`.

The host resolves the native window only from the authenticated session. A
protocol worker transfers one pending focus request through a five-second,
one-request UI-thread mailbox. The owning UI thread calls direct
`SetForegroundWindow` for that one window. A successful protocol response
means only that Windows accepted the foreground request; it does not reveal the
prior foreground window, the resulting focus, activation, z-order, input,
monitor, window handle, or whether a person noticed it. A refused or unavailable
native call reports only `window.unavailable`; a concurrent request reports
`window.busy`.

Installed record version 1.9 is the first record version allowed to name the
new `window.focus` grant. Earlier record versions that name it remain invalid.

## Consequences

- Applications gain a safe way to request attention for their own session
  window, using direct User32 rather than a browser or runtime dependency.
- Windows remains the authority on whether attention can be stolen; Anodrel
  does not add `AllowSetForegroundWindow`, input injection, a retry loop, or a
  workaround for a refusal.
- The command stays disjoint from UI Automation focus: it activates a native
  host window but does not name or move a semantic element, while UI Automation
  remains inaccessible from the application protocol.

## Revisit conditions

Revisit before adding a focus state read, event, subscription, native or
application-supplied target, `AllowSetForegroundWindow`, input simulation,
retry policy, window creation, fullscreen, geometry, or cross-window routing.
Each changes the authority or observation boundary and needs its own decision.
