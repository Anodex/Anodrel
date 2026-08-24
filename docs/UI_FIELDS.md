# Anodrel Text Fields

**Status:** Contract for the first node a person can type into. The host owns
the text and the caret; an application never sees a keystroke.

## Why this is the hard one

Every other node in `docs/UI.md` is something the application says to the
person. A field is the first one where the person says something back, and it is
where they type the things worth stealing: names, addresses, recovery codes,
whatever a form asks for.

That inverts the trust question the platform has answered so far. Until now the
risk was an application reaching outward — to a file, a window, the clipboard.
Here the risk is an application reaching *inward*, to what someone is typing,
before they have decided to send it.

So the rule this contract exists to establish:

> **An application learns a value, never the typing.**

A person who types `hunter2`, thinks better of it, deletes it, and types
something else has told the application exactly one thing: the something else.
The deleted text, the order it was typed in, the pauses, and the corrections are
not the application's to have.

## Ownership

| Thing | Owner |
| --- | --- |
| The current text | The host |
| The caret and selection | The host |
| Keyboard input | The host's UI thread |
| The initial value | The application, once, in the document |
| Reading the value back | The application, through a granted operation |

The application proposes a field's starting text when it publishes a document.
After that the host's copy is the truth. A later document replacement that
carries a different initial value **replaces** what the person typed, which is
the same rule every other node follows: a document is a whole snapshot, not a
patch.

## What a field carries

| Property | Rule |
| --- | --- |
| `id` | The existing validated `ElementId`; the only identity in an event. |
| `label` | Required, validated like any other visible text. A field with no label cannot be announced by a screen reader. |
| `value` | The initial text. May be empty. Bounded as below. |
| `placeholder` | Optional hint shown while the value is empty. Never returned as a value. |
| `maxLength` | 1 through 4,096 characters. |
| `enabled` | A disabled field is announced as unavailable and cannot be focused or typed into. |

Text is validated by the same rule as every other visible string: no control
characters. A field is one line, and unlike a text run it does not wrap: a
caret has to sit somewhere definite, and reflowing under someone mid-sentence
would move it. Multi-line input is a separate node with its own layout,
scrolling, and caret questions, and is not this.

### No secret fields in v1

There is no password or masked field, and that is a decision rather than an
omission.

A masked field is a promise: it says *what you type here is protected*. Anodrel
cannot honour that promise yet. The value would sit in a host buffer, cross the
protocol as ordinary text when read, and land in an application's memory with no
handling rule attached. Masking the pixels while doing all of that would be
security theatre — the worst kind, because a person would trust it.

`docs/CREDENTIALS.md` already defines how a secret is stored and retrieved
without an application ever choosing where it goes. A password field belongs in
that lineage, with its own decision, not as a `secret: true` flag on this one.

## Reading a value

A value crosses to the application only through a granted operation, never on a
timer and never as a side effect of anything else.

Protocol **1.15** defines that operation:

| Field | Value |
| --- | --- |
| Operation | `ui.fields.read` |
| Payload | `{ }` — exactly this, no field selector |
| Grant | `ui.fields.read` |
| Success | `{ "fields": [ { "id": string, "value": string }, ... ] }` |
| Errors | `ui.fields.unavailable` |

It reads a **snapshot**, not a stream: the current value of every field on the
current surface, at the moment of the request. There is no change event, no
keystroke event, and no subscription, because each of those hands over the
typing rather than the value.

### Why it takes no selector

The payload is empty. An application cannot name which fields to read, and gets
all of them or none.

That is not a convenience decision. A selector is a question, and asking
questions one at a time is how a stream gets rebuilt out of snapshots: polling
`{"id": "password"}` in a loop reconstructs the typing at whatever resolution
the caller likes. Returning the whole surface at once makes each read cost the
same, so there is nothing to gain by reading often.

It also means an application can only ever learn about **its own current
document's** fields, because that is the only surface the host will answer for.

### What it does not carry

No caret position, no selection, no character count separate from the value, no
timestamp, no indication of whether a field was edited, focused, or touched.
Those describe the typing. An empty field and an untouched field are reported
identically, because the difference between them is behaviour, not content.

Installed record version **1.5** adds `ui.fields.read` as a strict superset of
1.4. A record written for an earlier version that names the grant stays invalid.

The separate `init-form` development template exercises that contract with one
fixed field. Its generated app asks for a snapshot only after the person
activates its fixed submit action, does not echo or retain the value, and closes
its own session. See `docs/NATIVE_FORM_TEMPLATE.md` and Decision 0095.

## What a field must never become

- A key logger. No node reports individual keystrokes, and no operation returns
  input timing.
- A way to read another field, another session's fields, or a value the person
  never finished entering somewhere else.
- A clipboard reader. Pasting into a field is the person's action and puts text
  in the host's buffer; it does not grant `clipboard.read`.
- A native edit control. The host draws and handles the field itself, so there
  is no window handle, subclass, or message hook an application could reach.

## Accessibility

A field publishes as an `Edit` control with its label as its name, so
`docs/ACCESSIBILITY.md`'s UI Automation support announces it like any
other element. A matching visible field now also exposes its current host-owned
text through a read-only `IValueProvider` snapshot for Windows. That value
remains outside the application protocol; automation cannot set it or learn a
caret, selection, edit history, or change event. Because a UIA client can retain
a returned value, this does not change the v1 rule that fields are never secret
or masked. Decision 0071 defines the boundary.

## Reaching a field

A field takes focus by **click or by Tab**, and both are required: Tab alone is
not how anyone expects to use a text box. Clicking one moves the caret into it
and produces no semantic event — a click that lands on a field tells an
application nothing, exactly as tabbing to one does.

Clicking an action both focuses and activates it, because pressing a control is
also how a person expects to focus it. That is why the host asks two separate
questions of a click: *what should now have focus*, which a field can answer,
and *what did this activate*, which only an action can.

## Verification

Portable unit tests cover the bounds, the control-character rule, layout and hit
testing, focus participation, and the strict document codec in both directions.
Host tests cover typing, caret movement, and that a disabled field refuses
input. A manual check is in `docs/DEVELOPMENT_DIAGNOSTICS.md`.

## Compatibility

`docs/UI_DOCUMENTS.md` gains one node kind. Adding masking, multi-line input, a
change event, input timing, a native control, or any read that is not an
explicit granted snapshot requires its own decision, threat-model entry, and
protocol version. Decision 0067 records the reasoning.
