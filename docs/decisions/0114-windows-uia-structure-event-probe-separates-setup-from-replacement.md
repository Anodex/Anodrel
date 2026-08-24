# Decision 0114: Windows UI Automation structure-event probe separates setup from replacement

**Status:** Accepted

**Date:** 2026-08-24

## Context

Decision 0076 defines one outbound
`StructureChangeType_ChildrenInvalidated` notification after the direct Windows
host applies a newer authenticated UI-session document. Unit tests prove its
emission gate and source. They do not prove a real Windows UI Automation client
can register for the event before the first document replaces the host waiting
surface, receive it, and observe its fixed root source.

Registering an event handler after a development child has already published
its only document would make this check timing-dependent. Attempting to arm
that handler while the window observer blocks the UI thread is also invalid:
Windows must deliver the UI Automation query through the normal window message
loop before a direct client can obtain the root. Giving a child listener
readiness, an event result, or a protocol operation would create exactly the
application-visible event surface Decision 0076 rejects.

## Decision

Add one development-only
`--uia-structure-event-probe <native-client.exe>` route and one compiled,
two-document native diagnostic. It starts that child through the normal
fixed-grant authenticated session path. The child publishes one fixed initial
document containing only a fixed `prepare` action, then waits through the
ordinary revision-bound action-read path.

Once the normal UI message loop applies that initial document, a private MTA
worker obtains the host-owned root and registers one
`IUIAutomationStructureChangedEventHandler` on it with element scope. The
worker obtains the standard client-side Invoke interface for only the fixed
`prepare` action before registration, then registers and arms the structure
handler before calling that already-selected interface once. The child receives
that ordinary action, publishes a fixed replacement document containing only a
fixed `complete` action, and waits through the same action-read path.

The listener waits for exactly one event whose sender is `anodrel.surface`,
whose change kind is `ChildrenInvalidated`. Decision 0076 separately fixes the
provider call's runtime-ID pointer to null with length zero. The callback's
`SAFEARRAY` representation belongs to Windows and is not read or used to infer
that provider input. The worker then invokes the fixed `complete` action solely
so the compiled child performs its normal session close.

The prepared Invoke interface is held only by the host worker and is consumed
once after the listener is armed. It is not handed to the child, retained by
the production host, or represented in an application-facing interface.

The readiness signal, handler, sender identifier, change kind, window handle,
and result remain inside the short-lived host diagnostic.
No protocol field, capability, SDK method, child input, event listener,
delivery acknowledgement, or assistive-technology presence signal is added.
The handler unregisters and releases before its worker reports a fixed result.

## Consequences

Positive:

- a real Windows client proves registration, event delivery, the exact event
  kind, root source, and the existing authenticated document-to-window route;
- the event is caused by a controlled second authenticated document
  replacement, not a timing-sensitive initial render;
- the startup ordering stays deterministic without expanding application
  authority; and
- the compiled child's normal semantic-action close proves the probe did not
  replace the session lifecycle with a test-only shortcut.

Tradeoffs:

- the direct client requires another small hand-written COM callback and two
  Windows vtable declarations; and
- the check proves one fixed replacement after an ordinary initial render, not
  repeated replacement, rejected/stale silence, Narrator speech, arbitrary
  event subscriptions, or application behaviour beyond the fixed child
  sequence.

## Revisit conditions

Revisit before exposing any listener, readiness state, event sender, change
kind, runtime ID, callback, delivery result, or assistive-technology presence
to an application; adding another event kind; or adding a non-Windows probe.
