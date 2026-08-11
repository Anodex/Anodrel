# Decision 0066: An application proposes a window title; the host composes it

**Status:** Accepted

**Date:** 2026-08-10

## Context

`ROADMAP.md` has carried "a public application window capability remains
deferred" since the multi-window foundation landed, and
`docs/WINDOW_LIFECYCLE.md` set the price of lifting it: a versioned protocol,
verified executable identity, explicit permissions, cancellation rules, and its
own threat-model extension.

The whole of that price does not have to be paid at once, and it should not be.
"Window management" is not one capability; it is a dozen, with very different
risks. Creating a window, closing someone else's, moving one under the pointer,
and saying what the current one is showing have almost nothing in common except
the word *window*.

The last of those is the one an application cannot do without. Every window
Anodrel creates is titled by the host, so an application cannot say which
document is open. It is also, on inspection, the one with a threat nobody would
guess from its size.

A window title is not decoration. It appears in the task switcher, the taskbar,
window lists, screen-reader announcements, and screenshots — the places a person
looks to decide **what they are talking to**. An application that could write the
whole title could write `Windows Security`, and the operating system would render
that claim in its own trusted furniture, next to a real system dialog, at the
moment someone is deciding whether to type a password.

## Decision

Ship exactly one window capability: an authenticated session may **propose** the
title of the one window it already owns, and the host composes what is displayed.

**Composition, not assignment.** The host builds the final caption as
`<proposal> — <host-validated display name>`. The display name comes from the
machine-validated installed record, never from the request, the package content,
or anything the application influences at run time. The separator and suffix are
appended after validation, so a proposal cannot suppress, duplicate, or forge
them. `Windows Security` becomes `Windows Security — Anodrel Sample`: a window
that has told the truth about itself. Where no validated display name exists the
proposal stands alone, because an absent claim is safer than an unfounded one.

**No target field.** The request names no window, handle, or identifier. The
host resolves the window from the authenticated session, the same rule that
stops `session.close` reaching somebody else's window. A capability with no way
to name a victim cannot be aimed at one, and that is a stronger guarantee than
any check on a supplied target.

**No control characters at all, including a line feed.** Every surface renders a
title as one line. A newline or an escape could split one window's title into
what reads as two, or push the visible text away from the host's suffix — the
same impersonation the composition rule exists to prevent, arriving through the
character set instead of the string. This is stricter than the notification
body, which permits `\n`: a body is a paragraph and a title is a label.

**Write-only.** Nothing reads a title back. A read would hand the application a
way to probe the host's framing, and it already knows both halves of what it
would be told.

**One pending proposal, applied from the owning UI thread.** A protocol worker
never calls User32, so the proposal crosses a per-session mailbox and the UI
thread performs the call, mirroring the notification bridge. `window.busy` and
`window.unavailable` stay distinct answers: try again, versus this host has no
window to title.

## Consequences

An application can finally say what it is showing, which is the difference
between a diagnostic surface and something a person could use. It still cannot
learn that it has a window, where the window is, how large it is, or that any
other window exists.

The composition rule means the caption is never exactly what the application
asked for. That is the point, and it will surprise somebody: an application
proposing `Report.pdf` sees `Report.pdf — Anodrel Sample`. The alternative is a
title an application controls completely, which is the thing worth refusing.

Protocol 1.14 and installed record version 1.4 both grow by exactly one entry,
and a record written for an earlier version that names `window.title` stays
invalid, so provisioning cannot widen a record by accident.

The bound is 96 UTF-16 code units for the proposal, measured the way the native
call counts, so a value that validates never needs truncating on its way out.

## Alternatives considered

**Let the application set the whole title.** What every other desktop framework
does, and the reason a malicious page can put `Sign in — Microsoft` in a window
list. Anodrel's premise is that the host owns what the operating system is told
about an application; a freely writable title contradicts it directly.

**Prefix instead of suffix** — `Anodrel Sample — Report.pdf`. Unforgeable in the
same way, and worse in practice: the task switcher truncates from the right, so
every window from one application would show the same first few words and the
part that distinguishes them would be the part that disappears.

**Sanitise the proposal instead of composing.** Strip anything that looks like an
impersonation attempt. Every such filter is a guess about what a name means, in
every language, against an attacker who chooses the string. Composition needs no
guess: the true name is appended whatever the proposal says.

**Return the composed title on success.** Convenient for a client that wants to
display its own caption. It also hands out the host's framing format for probing,
and the application can already derive both halves. Refused for now; a read is a
separate capability if a real need appears.

**Wait and ship window management whole.** The capability an application needs
most would then be blocked behind the ones with the largest threat surface —
creation, targeting, and geometry. Shipping the narrow one first is what keeps
each risk reviewable on its own.
