# Decision 0193: Development native notification templates stay one-way

**Status:** Accepted

**Date:** 2026-09-03

## Context

Protocol 1.13 already gives an invited session one bounded `notification.show`
operation over direct Windows Shell32. The existing development diagnostic
requires a Node.js client, while the native templates exercise related UI
surfaces through constrained first-party Rust projects.

Adding `notification.show` to a regular, menu, form, context-menu, tray, or
window-control template would silently widen an unrelated executable's
authority. Letting a generator caller choose notification text, a duration,
an icon, an Application User Model ID, callbacks, or a second notification
would broaden the already accepted one-way notification contract.

## Decision

Keep every existing generator command and host route unchanged. Add one
operator-selected development path:

- `anodrel-native-app-tool init-notification <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-notification-template-client <client.exe>`.

The host creates one development session with exactly:

- `ui.document.write`;
- `notification.show`; and
- `session.close`.

The generated program has one compiled-in document, one compiled-in title and
body, and one fixed five-second observation period after the host accepts its
notification. It then requests only its own session close. It has no event
reader, notification ID, replacement, action, callback, toast identity, icon,
sound, schedule, configuration, network access, package identity, installer,
or signing behavior.

The host retains the Shell32 entry, host artwork, hover text, callback message,
native window, pipe worker, application attribution, and session cleanup.
Success means only that the host handed the fixed text to Shell32; neither the
host nor the generated app can learn whether a person saw it.

## Consequences

- Developers can manually check the existing one-way Windows notification
  route without Node.js, raw protocol JSON, or an external UI library.
- Existing templates retain their smaller fixed grant sets.
- The generated-child integration test proves private transport, document
  replacement, handover to the notification mailbox, and self-close. A person
  seeing the desktop notification remains a separate manual check.

## Revisit conditions

Revisit before adding configurable text, multiple notifications, callbacks,
actions, replacement, revocation, icons, sound, scheduling, application
identity, toast notifications, non-Windows adapters, or product packaging.
Each changes authority, privacy, or lifecycle and needs its own contract,
threat-model update, and verification.
