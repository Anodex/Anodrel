# Decision 0073: UI Automation focus remains host-owned

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel's Windows UI Automation provider can report the host's current
keyboard-focus snapshot. That lets a screen reader describe where a person is,
but it cannot move to a field or button. A custom-drawn native surface therefore
remains materially harder to use with assistive technology than one with native
controls, even though the same field and action are reachable with Tab and a
pointer.

`IRawElementProviderFragment::SetFocus` is the UI Automation operation for
moving focus inside a custom fragment. It must not become a new application
input path. In particular, an automation request must not name another window,
send a native input message, execute an action, expose focus to an application,
or leave a stale provider able to focus a node after the document changed.

## Decision

Implement `SetFocus` only for a visible, enabled, keyboard-focusable child in
the same immutable provider snapshot. The provider carries a private,
per-window focus route rather than a window-management API.

The route contains only:

- one document revision for an authenticated session, or no revision for the
  fixed diagnostic UI Lab;
- one bounded semantic element ID already published by that provider; and
- a one-request response slot shared with the owning UI thread.

The provider posts a private, payload-free host message after it creates a
route request. The message does not carry an address, an element ID, or an
application value, so another process cannot inject focus by sending it. It
only causes the owning view to look for its own pending request. When the call
originated on the UI thread, the host dispatches that same private message
synchronously rather than waiting for a message loop it is already running.

The owning UI thread revalidates the revision and target against its current
layout before it changes `UiFocus`. It accepts a target already focused as a
successful no-op. A stale revision, clipped target, disabled target, unknown
ID, missing view, full route, unavailable UI thread, or expired request fails
closed. One request waits at most 250 milliseconds; a timeout removes its exact
slot so a late completion cannot answer a later request, change focus, or leave
the route busy.

Windows UI Automation focuses the fragment's parent before calling
`SetFocus`, so Anodrel does not call `SetForegroundWindow`, synthesize input,
or otherwise steal focus. Its only successful effect is the host's internal
focus state and the matching repaint.

There is no protocol operation, capability grant, installed-record version,
application callback, event, focus readback, or accessibility-presence signal.
An application can still neither request focus nor learn whether assistive
technology moved it.

## Consequences

- Screen readers and UI Automation clients can move to the same fields and
  buttons that local keyboard traversal can reach.
- The UI thread remains the sole writer of live view state; a provider holds a
  bounded request route, never a mutable view, registry entry, or native input
  handle.
- A provider created for an earlier session document cannot focus the later
  document, even if an element ID was reused.
- Existing focus reporting stays snapshot based. A provider changes its own
  copied focus result only after its `SetFocus` request succeeds; it does not
  begin chasing arbitrary later live focus changes.

## Alternatives considered

**Expose focus through the application protocol.** Rejected. It would let an
application manipulate a native window's keyboard target and observe a
person's navigation, neither of which is needed for assistive technology.

**Call `SetForegroundWindow` from the provider.** Rejected. UI Automation
already focuses the containing fragment before `SetFocus`; forcing foreground
activation would make an assistive-technology request able to interrupt another
application.

**Pass an element pointer through a custom Windows message.** Rejected. A
window message is externally sendable. A payload-free wakeup plus a host-owned
one-request route has nothing untrusted to dereference or interpret.

**Write focus directly from the automation caller.** Rejected. It would give a
COM client thread mutable access to a view that the Windows UI thread owns,
break the host's layout-validation boundary, and race document replacement.

## Revisit conditions

Revisit before adding focus-change events, application focus readback or
control, automation editing, caret or selection reporting, text ranges,
hierarchical groups, or a non-Windows accessibility adapter.
