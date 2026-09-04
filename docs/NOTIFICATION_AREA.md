# Windows notification-area foundation

**Status:** Host-only Windows resource foundation. It is not yet a public tray
API or application capability.

## Purpose

Windows uses one `Shell_NotifyIconW` entry both for the legacy information
balloon used by Anodrel notifications and for a future system-tray surface.
`anodrel-windows-notification-area` owns that native entry separately from
either higher-level feature:

~~~text
host-owned window + host-owned icon + host-owned tooltip
                              |
                              v
                 one Shell32 notification-area entry
                              |
              +---------------+---------------+
              |                               |
     bounded notification balloon      future semantic tray surface
~~~

This keeps one native presence, one UI-thread owner, and one cleanup lifetime.
An application never receives the icon identifier, window handle, callback
message, native status, or image handle.

## Current boundary

The adapter creates an entry only for a nonzero host-owned window. Its tooltip
is nonempty, control-free host text of at most **127 UTF-16 code units**, the
exact `NOTIFYICONDATAW` field capacity less the terminator. It either fits or is
rejected; host text is never silently truncated.

The entry currently supports the bounded silent information balloons required
by [notifications](NOTIFICATIONS.md). The title and body limits remain owned by
`anodrel-notifications`, which validates values before this lower-level adapter
receives them. A Windows host may later add one of its own private callback
messages to an existing entry; that safe adapter method does not dispatch,
report, or expose callbacks. Shell32 operations run only on the UI thread.
Dropping the entry calls `NIM_DELETE` best-effort so a session cannot leave a
stale icon behind.

The direct Windows structs, fixed fields, and FFI calls are contained in one
`raw` module. Its tests pin the real structure size and UTF-16 behavior;
the safe wrapper tests host-tooltip validation and keeps native window material
out of debug output.

## Deliberately absent

There is no tray protocol operation, capability, model, callback delivery,
click event, context menu, icon selection, tooltip selection, window toggle,
or user-attention readback in this adapter. The existing `notification.show`
path stays a one-way announce with no action or delivery result.

A later tray model must be versioned and semantic. It must use this entry
rather than create a second icon, keep native command numbers private, and
deliver only a bounded revision-checked semantic action through the established
session event path. See Decision 0190.
