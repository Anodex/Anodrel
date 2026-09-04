# Decision 0192: Development native tray templates stay explicit

**Status:** Accepted

**Date:** 2026-09-03

## Context

Protocol 1.33 adds a semantic notification-area tray model to the direct
Windows host. The regular native template, menu template, and context-menu
template all have smaller distinct grant sets. Broadening any existing route
with `tray.write` would silently let an unrelated generated executable create
persistent visible desktop presence for the duration of its session.

Letting generator callers supply tray items, callbacks, icons, tooltips,
coordinates, native command values, capabilities, or a background lifecycle
would defeat the contract that makes tray state host-owned and semantic.

## Decision

Keep every existing generator command and host route unchanged. Add one
operator-selected development path:

- `anodrel-native-app-tool init-tray <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-tray-template-client <client.exe>`.

The host creates one development session with exactly:

- `ui.document.write`;
- `ui.events.read`;
- `tray.write`; and
- `session.close`.

The generated program has one compiled-in document, one compiled-in tray menu
with one enabled semantic action, and one typed tray-action read loop. It has
no icon, tooltip, callback, coordinate, window handle, command number,
background mode, close-to-tray behavior, capability declaration, configuration
loader, network access, package identity, installer, or signing behavior. The
host retains the Shell32 entry, callback, User32 popup, pointer placement,
process lifetime, pipe, and shutdown facts.

## Consequences

- Developers can visibly exercise the complete first-party tray path without a
  browser runtime, Node.js, raw protocol JSON, or external UI library.
- Existing templates retain their smaller fixed capability sets.
- The generated-child integration test proves private transport, model
  replacement, revision-bound semantic action, and self-close without
  synthesizing a click. A real notification-area right-click remains manual.

## Revisit conditions

Revisit before adding caller-provided tray content, dynamic tooltips, icons,
badges, separators, submenus, checked state, left-click command behavior,
background lifetime, close-to-tray, a global shortcut, callback, coordinate,
native handle, non-Windows adapter, product identity, packaging, or signing.
Each changes authority, privacy, or lifecycle and needs its own contract,
threat-model update, and verification.
