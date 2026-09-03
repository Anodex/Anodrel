# Decision 0190: Notification-area lifetime is separate from notification text

**Status:** Accepted

**Date:** 2026-09-03

## Context

The existing Windows notification adapter correctly creates one Shell32
notification-area icon for the duration of an interactive session, then uses
that icon to deliver bounded information balloons. A native system-tray surface
would use the same Windows resource but needs different behavior: a persistent
host-owned tooltip, callback routing, and a semantic menu.

Adding a second icon for tray behavior would look broken, needlessly consume
notification-area space, and give two independent lifetimes to what Windows and
the user perceive as one application presence. Adding menus and callback logic
directly to the notification-text adapter would instead blur a small one-way
portable service with Windows-native resource ownership.

## Decision

Extract the direct Shell32 notification-area entry into
`anodrel-windows-notification-area`. It owns exactly one entry for one host
window: host-selected icon, validated host tooltip, bounded information balloon
display, and best-effort removal on drop. Its direct bindings stay in a focused
`raw` module.

`anodrel-windows-notifications` becomes a thin adapter over that entry. Its
public portable notification contract, `notification.show` capability, UI-thread
mailbox, values, error categories, and existing behavior remain unchanged.

The extracted adapter is host-only infrastructure, not a tray capability. It
accepts no application protocol input, exposes no native handle or callback,
and does not yet create a menu or add a second icon. A later tray surface must
share this one resource through an explicit host composition boundary.

## Consequences

- Notification behavior remains stable while the native resource is organized
  around its true lifetime.
- Future tray work can share one icon rather than overlaying another Shell32
  entry beside notifications.
- Direct unsafe bindings, fixed UTF-16 fields, and Drop cleanup have one small
  reviewable home.
- A notification remains one-way. This decision does not introduce click,
  dismissal, focus, or user-attention observation.

## Revisit conditions

Revisit when a versioned tray model, native callback route, product identity
tooltip, toast-notification identity, multi-session host, or non-Windows
notification-area mapping is defined.
