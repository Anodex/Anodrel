# Decision 0062: Notifications start as a one-way bounded announce over Shell32

**Status:** Accepted

**Date:** 2026-08-09

## Context

Notifications are one of the Phase 2 platform services listed in `ROADMAP.md`
and the only one in that line with no contract at all. Clipboard, external
links, file dialogs, paths, storage, and credentials each began as a deliberately
narrow foundation; notifications need the same treatment before an application
can ask for one.

Two choices have to be made before any code exists: how much a notification can
say and do, and which Windows surface delivers it.

The second choice is the constrained one. Windows toast notifications through
`ToastNotificationManager` are the richer and more modern surface, and they
require an Application User Model ID backed by an installed Start Menu shortcut
or a packaged identity. Anodrel has neither. Packaging and installation are an
explicitly open decision in `docs/ARCHITECTURE_FOUNDATIONS.md`, and the only signed
application the platform can currently provision is the development fixture from
Decision 0061.

That leaves two honest options: invent a packaging identity as a side effect of
building a notification feature, or ship a notification path that would silently
do nothing on the unpackaged hosts Anodrel actually runs on today.

## Decision

Define notifications as a **one-way bounded announce**, and map the first
implementation onto `Shell_NotifyIconW` with `NIF_INFO`.

A notification carries exactly a title and a body, validated as 1 to 63 and 1 to
255 UTF-16 code units respectively, with control characters rejected and line
feeds allowed only in the body. The single operation reports whether the host
accepted the values and nothing else.

There is no identifier, replace, update, revoke, progress, dismiss, action,
inline reply, image, sound, icon selection, scheduling, grouping, persistence,
or callback. There is no read surface at all, so `notification.show` is the only
capability and it has no counterpart to grant.

Accepted means handed over, not seen. An application cannot learn that the user
has notifications silenced, is in a focus mode, or has muted it, and a refusal
never distinguishes a muted application from a busy shell.

Delivery reuses the established UI-thread bridge. Shell32 is subject to the same
rule that keeps a pipe worker away from User32, so the authenticated worker
places one bounded request in a per-session mailbox and the owning native UI
thread performs the call, exactly as file dialogs already work.

The notification icon is host-owned and generated from the brand crate at run
time, like the window icon. An application cannot supply, select, or replace it.

## Consequences

Positive:

- the contract works today on an ordinary desktop host, with no identity
  Anodrel cannot honestly claim and no runtime dependency;
- the portable limits come from the narrowest real mapping, so a value that
  validates never needs truncating on its way to the operating system;
- with no identifier, no callback, and no read surface, a notification cannot
  become a side channel, a tracking mechanism, or a way to observe the user's
  attention state;
- a later toast adapter can satisfy the same portable contract unchanged.

Tradeoffs:

- 63 and 255 UTF-16 code units are short, and an application with more to say
  must say it in its own window;
- the Shell32 surface is visually plainer than a toast and shows an icon in the
  notification area for the duration of the call;
- without a replace or revoke operation, an application that needs to correct a
  message can only send another one.

## Confirmed in use

The Shell32 mapping is the accepted native Windows baseline. It has been run on
Windows 11: a notification requested through the development diagnostic appeared
with the supplied title and body.

That run also showed the cost of the deferred packaging decision. Windows
attributes the notification to `anodrel-windows-host.exe` rather than to a
product name, because Shell32 has no application identity to use. Fixing that
needs the same Application User Model ID toast notifications need, so both wait
on the production identity and packaging decision, which is deferred
deliberately and recorded in `ROADMAP.md`.

## Revisit conditions

Revisit when Anodrel has a documented packaging and installation identity that
can carry an Application User Model ID, when an application genuinely needs
actions or replace semantics, or when a non-Windows host needs an equivalent
mapping. A richer surface must extend this contract rather than replace it, and
must keep the rule that an application cannot observe suppression.
