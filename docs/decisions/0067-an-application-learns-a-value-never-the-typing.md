# Decision 0067: An application learns a value, never the typing

**Status:** Accepted

**Date:** 2026-08-11

## Context

The portable UI model has four node kinds: a stack, a scroll viewport, a text
run, and an action. All four are things an application says to a person. None of
them lets the person say anything back beyond activating an action.

That is the binding constraint on the whole platform. An application on Anodrel
cannot render a text input box, so it cannot ask for a name, a search term, or a
file name. Everything above the platform — the SDK, the sample, an eventual
Anodex migration — is limited by it, and no amount of further work on transport,
launch, or rendering moves it.

Adding the node is not the hard part. Deciding what an application is allowed to
know about what someone typed is.

Every capability so far has managed an application reaching *outward*: to a
file, a window, the clipboard, the notification area. A field is the first one
where the risk points *inward*. A person typing into an application's window is
producing exactly the material worth stealing, and they are producing it before
they have decided to send it. A half-typed password, a corrected address, a
recovery code pasted and then thought better of — all of it exists, briefly, in
a buffer, and the question is who may see it.

The web answers "the page, on every keystroke". That answer is why a login form
can exfiltrate a password the user never submitted.

## Decision

**An application learns a value, never the typing.**

The host owns the text, the caret, and the selection. Keyboard input is handled
on the host's UI thread and never leaves it as input. An application supplies a
field's initial value in a document and, later and separately, may ask for the
current value through a granted operation that returns a **snapshot** — the text
as it stands at that moment.

What that rules out, deliberately:

- **No change event and no subscription.** Either would deliver the typing one
  fragment at a time, which is the thing being refused, just spelled slowly.
- **No keystroke or timing information.** Not the keys, not the order, not the
  pauses. Typing rhythm identifies people.
- **No read of a value the person is still editing elsewhere**, no read of
  another session's fields, and no read that arrives without the grant.

A person who types something, deletes it, and types something else has told the
application one thing: the something else.

**Focus and input stay host-side.** The host draws the field and handles its
keys itself, so there is no native edit control, window handle, or message hook
for an application to reach. Pasting is the person's action into the host's
buffer and does not grant `clipboard.read`.

**No secret field in v1.** A masked field promises *what you type here is
protected*, and this platform cannot honour that promise yet: the value would
sit in a host buffer, cross the protocol as ordinary text, and land in an
application's memory with no handling rule attached. Masking the pixels while
doing all that would be theatre, and the dangerous kind, because someone would
trust it. `docs/CREDENTIALS.md` already stores and retrieves secrets without an
application choosing where they go; a password field belongs in that lineage,
with its own decision.

**Display and input before readback.** The node lands with no read operation at
all. That is a coherent state rather than a half-built one: a host surface can
collect input and act on it locally, and the protocol surface arrives with its
own grant and threat-model entry when it does.

## Consequences

The platform gains the node that unblocks everything above it, and gains it with
the readback question answered before any value can cross.

An application that wants live validation — "this email looks wrong" as you
type — cannot have it. That is a real cost, and it is the cost of the rule. A
snapshot read on an explicit action covers submitting a form, which is what most
applications actually need; live per-keystroke feedback would require handing
over the typing, and that is exactly what this refuses.

A document replacement carrying a different initial value overwrites what the
person typed. That follows from documents being whole snapshots rather than
patches, and it is worth stating because an application that republishes its
document on a timer would erase input. The alternative — preserving host state
across a replacement — would mean the application's document and the person's
text disagree, with no rule for which wins.

Accessibility inherits the existing one-directional contract: assistive
technology reads the field's presence and label and cannot read its value or
move focus into it. That is consistent, and it is also a genuine limitation for
a screen-reader user filling a form, which `docs/ACCESSIBILITY.md` will have to
revisit as its own decision rather than by loosening this one quietly.

## Alternatives considered

**A change event.** What every UI toolkit provides, and the reason a web page
sees a password before it is submitted. It delivers the typing in fragments; the
rule above exists to refuse exactly that.

**Let the application own the value and echo it back.** The application controls
the text, the host renders what it is told. Simple, and it forces a round trip
per keystroke — the typing, again, plus a visible input lag on every character.

**Ship a masked field now and treat the value carefully later.** The promise a
mask makes is the one part that cannot be added retroactively: anyone who typed
a password into it in the meantime already typed it into a plain buffer.

**Keep host text across document replacement.** Avoids erasing input on a
republish. It also creates two disagreeing sources of truth with no rule for
which wins, and the document model's whole premise is that a document is a
complete snapshot.
