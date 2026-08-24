# Anodrel UI Automation structure events

After the UI thread accepts a newer authenticated UI-session document, Anodrel
raises one best-effort `ChildrenInvalidated` structure event from the window
root. This says its full child subtree was replaced.

~~~text
accepted newer document → UI-thread view replacement → fresh root provider
                                                       → ChildrenInvalidated
~~~

Anodrel supplies no runtime-ID payload to `UiaRaiseStructureChangedEvent`: its
pointer is null and its length is zero. It has no listener check, subscriber,
log, result, application callback, protocol operation, or capability. An
application cannot request it or learn whether assistive technology received
it. No event is raised for stale or absent documents, paint, layout, resize,
typing, field changes, focus, actions, dialogs, notifications, or closure.

## Automated Windows acceptance

`--uia-structure-event-probe` passed on Windows on 2026-08-24. It uses one
compiled two-document child: a private direct client arms an element-scoped
listener on the first authenticated document, invokes its fixed prepare action,
then accepts only a `ChildrenInvalidated` callback whose sender is
`anodrel.surface`. The child then receives a second fixed action and closes its
own session. This proves one real authenticated replacement and Windows event
delivery without adding a listener, callback, readiness signal, or result to
the application boundary.

The provider-call runtime-ID input is unit-tested separately. The callback's
runtime-ID representation belongs to Windows and is deliberately not read by
the probe; its fixed source and event kind are the portable assertion here.
Run it with the command in `docs/UI_AUTOMATION_STRUCTURE_EVENT_PROBE.md`.

## Manual verification

Register a UI Automation structure-changed handler on an authenticated session
before a later accepted document replacement. Expect exactly one
`ChildrenInvalidated` event from `anodrel.surface`; refresh its children and
confirm the accepted document. Repeated polls without a new document, typing,
focus movement, resize, and actions must produce none.

This manual Inspect or accessibility-client check remains pending. The
repeatable fixed route above is separate evidence. See Decision 0076.
