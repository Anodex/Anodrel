# Decision 0071: UI Automation field values are read-only host snapshots

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0067 gives each native surface host-owned `UiFieldStates`. A person
can type into a visible field, while an application can learn the resulting
value only through its separately granted whole-surface snapshot operation. The
Windows UI Automation provider currently announces a field's label and role but
not its current value. That makes a form materially less usable with a screen
reader.

The omission protected an important boundary while it was unsettled: an
accessibility adapter must never become an alternate application protocol, a
keystroke stream, or a remote editing route. It is now possible to add the one
missing read without relaxing any of those rules. Windows defines
`IValueProvider` for a single-line control whose value is a string; Anodrel
fields are exactly that shape.

## Decision

For each `WM_GETOBJECT` query, the host copies the current text of its
`UiFieldStates` alongside the same layout-derived semantic snapshot and focus
snapshot already used to build the UI Automation tree. The provider reduces
those values to the visible published `Edit` elements in that one tree. A
missing, non-field, clipped, or filtered ID has no value pattern.

Each matching `Edit` exposes `IValueProvider` and the two corresponding UI
Automation properties:

- `Value` is the copied current field text; and
- `IsReadOnly` is always true **to UI Automation**.

`SetValue` always returns `UIA_E_NOTSUPPORTED`, even for a host-enabled field.
Anodrel does not treat a UI Automation client as a person typing: it cannot
change field text, move a caret, select text, send a native input message, or
produce an application event. Local keyboard and pointer input remain the only
writers of host field state. A disabled field may still expose its visible value
as a read-only control; disabledness continues to be reported separately by
`IsEnabled`.

The value is an immutable provider snapshot. A fresh UI Automation query can
observe a later user edit, but an earlier provider neither reads the live window
registry nor follows subsequent edits. The provider returns a normal COM BSTR;
Windows and the client own its returned copy. Since a client can retain that
copy, this is deliberately not a secret-input facility. Anodrel v1 has no
password or masked field, and this decision does not add one.

Nothing crosses into an application through accessibility. There is no protocol
field, operation, grant, version, callback, event, listener check, UIA handle,
or tree-read route. `ui.fields.read` remains the sole application-facing route
and retains its whole-surface, separately granted, snapshot-only contract.

## Consequences

Positive:

- a screen reader can obtain the value a person currently sees in a visible
  single-line field;
- the host has one value owner and the provider obtains only a bounded,
  immutable copy of it; and
- automation cannot silently edit a form or turn host input into an
  application-visible typing stream.

Tradeoffs:

- assistive technology sees the field as read-only to automation, because
  `SetValue` is intentionally unavailable even though a person can type into
  an enabled field through the host;
- a retained provider or BSTR can describe an earlier value until the UIA
  client releases it; and
- selection, caret, text ranges, editing through automation, value-changed
  events, multi-line text, and secret fields remain deferred.

## Revisit conditions

Revisit before adding `SetValue`, reporting a caret or selection, text ranges,
value or text-change events, a masked field, multi-line input, a live registry
lookup, or any application-visible accessibility observation. Each changes a
separate privacy, authority, or lifetime boundary.
