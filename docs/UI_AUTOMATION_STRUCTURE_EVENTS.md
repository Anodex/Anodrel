# Anodrel UI Automation structure events

After the UI thread accepts a newer authenticated UI-session document, Anodrel
raises one best-effort `ChildrenInvalidated` structure event from the window
root. This says its full child subtree was replaced.

~~~text
accepted newer document → UI-thread view replacement → fresh root provider
                                                       → ChildrenInvalidated
~~~

It has no runtime-ID payload, listener check, subscriber, log, result,
application callback, protocol operation, or capability. An application cannot
request it or learn whether assistive technology received it. No event is raised
for stale or absent documents, paint, layout, resize, typing, field changes,
focus, actions, dialogs, notifications, or closure.

## Manual verification

Register a UI Automation structure-changed handler on an authenticated session
before its document replaces the waiting surface. Expect exactly one
`ChildrenInvalidated` event from `anodrel.surface`; refresh its children and
confirm the accepted document. Repeated polls without a new document, typing,
focus movement, resize, and actions must produce none.

This check is pending. See Decision 0076.
