# Anodrel Window Title

**Status:** The first public window capability. One bounded, granted, write-only
proposal, applied by the host to the session's own window.

## Purpose and boundary

Every native window Anodrel has created so far is titled by the host. That is
correct for a diagnostic surface and wrong for an application: a text editor
that cannot say which file is open is not a text editor.

This is the smallest capability that fixes it without opening window management.
An authenticated session may **propose** the title of the one window it already
owns. It may not:

- name, select, enumerate, or discover any window, including its own;
- obtain a window handle, identifier, position, or size;
- create, close, move, resize, focus, minimise, or restore a window;
- read back its window's title, or any other window's;
- affect a window belonging to another session, another application, or the
  host's own diagnostic surfaces.

There is no target field in the request. The host applies the title to the
window that the requesting session owns, resolved from the authenticated session
itself — the same rule that makes `session.close` unable to close somebody
else's window. A capability with no way to name a victim cannot be aimed at one.

`docs/WINDOW_LIFECYCLE.md` remains the contract for how windows are created and
destroyed; none of that becomes application-controlled here.

## The host keeps the last word on what a window says

A window title is not decoration. It appears in the task switcher, the taskbar,
window lists, screen-reader announcements, and screenshots — the places a person
looks to decide **what they are talking to**. An application that could write
the whole title could write `Windows Security` or the name of a competitor's
application, and the operating system would present that claim in its own
furniture.

So the application does not write the whole title. It writes a part, and the
host composes the rest:

~~~text
<proposal> — <host-validated display name>
~~~

The display name comes from the machine-validated installed record, never from
the request, the package content, or anything the application can influence at
run time. The separator and the suffix are added by the host after validation,
so a proposal cannot suppress, duplicate, or forge them. When a session has no
validated display name, the proposal is applied on its own rather than with an
unverified suffix — an absent claim is safer than an unfounded one.

The result is that an application can say what it is showing, and cannot change
what it is. A proposal of `Windows Security` on the sample application produces:

~~~text
Windows Security — Anodrel Sample
~~~

which is a window that has told the truth about itself.

## Validation

The proposal is validated before it reaches any native call.

| Rule | Value |
| --- | --- |
| Maximum length | 96 UTF-16 code units |
| Empty | Rejected |
| Control characters | Rejected, all of them |

Length is measured in UTF-16 code units because that is what the Windows call
counts; counting bytes or characters would let a value pass validation and still
need truncating on its way out.

**No control character is allowed, including a line feed.** A title is rendered
as one line by every surface that shows it. A value containing a newline, a
carriage return, or an escape sequence could split one window's title into what
reads as two, or move the visible text away from the host's suffix — which is
exactly the impersonation the composition rule above exists to prevent. Notice
that the notification body permits `\n` and this does not: the difference is that
a body is a paragraph and a title is a label.

A rejected proposal fails as `window.title_invalid`. **The failure never echoes
the offending text**, because an error message is another string that ends up in
logs and diagnostics, and text that was refused for being unsafe to display must
not be smuggled somewhere else to be displayed.

## Threading

A protocol worker never calls User32. The authenticated worker hands one
validated proposal to a per-session mailbox and the owning native UI thread
performs `SetWindowTextW`, mirroring the notification bridge in
`docs/NOTIFICATIONS.md`.

At most one proposal may be pending at a time. A second while one is in flight
is refused as `window.busy`, which is a different answer from
`window.unavailable`: busy means try again, unavailable means this host has no
window to title. A proposal the UI thread never completes fails safely once the
five-second bridge timeout elapses, and the session is freed rather than left
permanently busy by one stuck request.

Applying a title is a fast call, so the bridge blocks the worker rather than
coalescing. A session that wants to update a title rapidly is expected to do so
at a human rate; nothing here is on a frame path.

## Protocol

Protocol **1.14** defines one operation, gated on both the minor version and its
own grant:

| Field | Value |
| --- | --- |
| Operation | `window.title.set` |
| Payload | `{ "title": "<proposal>" }` — exactly this field |
| Grant | `window.title` |
| Success | `{ }` — acceptance only, no composed text returned |
| Errors | `window.unavailable`, `window.busy`, `window.title_invalid` |

Success means the host applied a title. It does not report what the composed
text became: returning it would hand the application a way to probe the host's
framing, and it already knows both halves of what it would be told.

Installed record version **1.4** adds `window.title` as a strict superset of
1.3. A record written for an earlier version that names the grant stays invalid,
so provisioning cannot widen a record by accident.

## Failure behaviour

| Category | Meaning |
| --- | --- |
| `window.unavailable` | This session has no host window to title, or the native call failed. |
| `window.busy` | Another proposal for this session is still pending. |
| `window.title_invalid` | The proposal failed the bounds or character rules above. |

No category carries a native status code, a window handle, a path, or the
proposed text. `window.unavailable` deliberately does not distinguish "no window
was ever created" from "the call failed", because the difference is host state
an application has no business learning.

## Verification

Portable unit tests cover the bounds at and beyond their limits, UTF-16
measurement against a surrogate pair, control-character rejection including the
line feed, the composition rule, and that a proposal cannot produce a title
whose visible application name differs from the validated one. Mailbox tests
cover single delivery, the busy answer, and the timeout freeing the session.

Protocol contract tests verify the exact payload shape, the independent grant
check, the version gate, and that no failure echoes the proposal.

Manual check on Windows is in `docs/DEVELOPMENT_DIAGNOSTICS.md`: a development diagnostic
proposes a title, the window's caption changes, and the application-name suffix
remains present and correct.

## Compatibility

`window.title.set` is complete as specified. Adding a read, a target, a second
window, a position, a size, or any other window property is a new capability
with its own grant, decision, threat-model entry, and protocol version — not an
extension of this one. Decision 0066 records the reasoning.
