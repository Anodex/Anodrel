# Anodrel notification foundation

**Status:** The portable values and the UI-thread bridge in
`anodrel-notifications` are implemented and tested. The direct Windows adapter
and a protocol capability are not. No application can reach a notification yet.

## Boundary

Anodrel's first notification service delivers one short, plain-text message to
the operating system's ordinary notification surface. It is a one-way announce:
the application hands over a title and a body and learns only whether the host
accepted them.

The portable service interface is deliberately small:

~~~text
NotificationService
  show(notification) -> accepted | NotificationServiceError
~~~

The service has no identifier, replace, update, revoke, progress, or dismiss
operation; no action buttons, inline reply, image, sound, or icon selection; no
delivery, click, or dismissal callback; no scheduling; no grouping; no
persistence; and no read access to any notification the user has received.

**Accepted means handed over, not seen.** The user may have notifications
silenced, be in a focus mode, or have muted this application entirely. The
platform reports what it did, never what the user experienced, and an
application cannot detect suppression. That is a deliberate privacy line, not a
gap to close later.

## Values and limits

A notification carries exactly two validated values:

| Value | Rule |
| --- | --- |
| `title` | 1 to **63 UTF-16 code units**. No control characters. |
| `body` | 1 to **255 UTF-16 code units**. Line feeds allowed; no other control characters. |

The limits are counted in UTF-16 code units because that is the unit the first
operating-system mapping actually measures, and counting it portably keeps a
value that validates from being truncated during conversion. They are small
because the target surface is small; an application that needs to say more
should say it in its own window.

Control characters are rejected so a notification cannot use carriage returns or
escape sequences to spoof a second message or misrepresent its source. A body
may contain line feeds because the target surface renders them as line breaks.

Both values are required. An empty notification would be a silent poke with no
content, which is a nuisance pattern rather than a feature.

## Authority

Showing a notification is a single host-issued capability, `notification.show`.
There is no read counterpart to grant, because nothing can be read.

The grant is machine-selected like every other: it comes from the installed
application record's capability array, and the capability is checked immediately
before the service is used. `docs/LAUNCH.md` records the record version that
adds it.

## Threading

A notification reaches the operating system through Shell32, and the same rule
that keeps a pipe worker away from User32 keeps it away from Shell32. The
authenticated worker therefore cannot show a notification directly.

Delivery uses the existing bridge shape: the worker places one bounded request
in a per-session `NotificationMailbox`, the owning native UI thread takes it
exactly once, performs the operating-system call, and completes it. This mirrors
`docs/FILE_DIALOGS.md`; the difference is that a notification returns no value
beyond acceptance.

One request may be pending at a time. A second request while one is pending is
refused as `Busy` rather than `Unavailable`, because those are different
answers: `Busy` means try again, `Unavailable` means this host cannot show
notifications at all.

The response timeout is **5 seconds**, far shorter than the file-dialog bridge's
two minutes. A dialog waits on a person deciding; a notification waits only on a
shell call that should finish in milliseconds, so a worker blocked longer than
that is stuck rather than patient. A timed-out request clears the session's slot,
so one stuck call cannot leave that session permanently busy.

A completion is ignored unless it names the active request **and** that request
has already been taken by the UI thread, so a response can never race ahead of
the call it claims to describe.

## Windows mapping

The first Windows adapter uses `Shell_NotifyIconW` from Shell32 with the
`NIF_INFO` flag: it adds a host-owned notification icon, supplies the validated
title and body, and removes the icon again. It uses only direct Windows APIs and
adds no runtime dependency.

The `NOTIFYICONDATAW` fields behind this mapping are fixed-size UTF-16 buffers —
64 units for the title and 256 for the body, each including a terminator — which
is where the portable limits above come from. The adapter never truncates: a
value that validated portably always fits.

The icon is host-owned and generated from the brand crate at run time, exactly
as the window icon already is. An application cannot supply, select, or replace
it, so a notification cannot impersonate another application's identity through
its artwork.

### Why not toast notifications

Windows toast notifications through `ToastNotificationManager` are the richer
surface, and they require an Application User Model ID backed by an installed
Start Menu shortcut or a packaged identity. Anodrel has neither: installation
and packaging are the open gate recorded in `docs/ARCHITECTURE.md`, and the only
signed application it can currently provision is a development fixture.

Adopting toast now would mean either inventing that packaging story as a side
effect of a notification feature, or shipping a notification path that silently
does nothing on an unpackaged host. The Shell32 mapping works today on an
ordinary desktop host, needs no identity Anodrel cannot honestly claim, and
keeps the portable contract small enough that a later toast adapter can satisfy
it unchanged. See Decision 0062.

## Failure behaviour

Failures return only safe categories:

| Category | Meaning |
| --- | --- |
| `Unavailable` | The host has no surface to attach a notification to, or the operating system refused. |
| `Busy` | Another notification request for this session is already pending. |

No category carries a native status, window handle, icon handle, path, or the
notification's own text. A refusal never distinguishes "the user has this
application muted" from "the shell was busy", because that distinction is the
user's business.

## Security and privacy

Notification text is application-controlled content displayed outside the
application's own window, so it is treated as untrusted display data:

- it is validated and bounded before any operating-system call;
- it never enters logs, diagnostics, crash output, or capability context;
- it cannot carry markup, links, script, a file path, or a native handle; and
- it cannot name or impersonate another application, because the title is text
  and the icon is host-owned.

The host attaches at most one notification icon at a time and removes it on
every path, so a failed call cannot leave a stale icon in the notification area.

## Verification

Unit tests in `anodrel-notifications` prove that both values are required, that
each is accepted exactly at its bound and rejected one unit beyond it, that
length is measured in UTF-16 code units rather than bytes or characters, that
control characters which could forge a second message are rejected, that a line
feed is allowed only in the body, and that neither failure category describes
the user's attention state.

Bridge tests cover the one-request handover, the refusal of a second request as
`Busy`, the rejection of a completion that is stale or arrives before the UI
thread has taken the request, and a request the UI thread never answers — which
must fail safely and leave the session usable again.

The remaining pieces each need their own verification before an application can
reach a notification: boundary tests for the Windows adapter and protocol
contract tests for the capability check.

## Deferred

Actions, inline reply, images, sound selection, progress, replace and revoke,
delivery and click callbacks, scheduling, grouping, history, per-application
consent UI, toast notifications, and non-Windows adapters are all outside this
foundation. Each needs its own contract, capability decision, threat-model row,
and tests.
