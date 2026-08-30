# Decision 0121: Development native context-menu templates stay explicit

**Status:** Accepted

**Date:** 2026-08-29

## Context

Protocol 1.32 adds a bounded, host-owned context-menu model to the direct
Windows session host. The regular native template deliberately has only
document write, semantic-action read, and self-close authority. The menu-bar
template has a different `menu.write` grant, local shortcut contract, and
activation path. Adding context-menu authority to either route would silently
broaden an existing generated executable.

Letting a generator caller choose items, coordinates, event payloads,
capabilities, native commands, or a context target would defeat the feature's
host-owned boundary and make the generated project a protocol bypass.

## Decision

Keep every existing generator command and host route unchanged. Add one
operator-selected development path:

- `anodrel-native-app-tool init-context-menu <destination> <project-slug>
  <display-label>`; and
- `anodrel-windows-host --native-context-menu-template-client <client.exe>`.

The host creates one development session with exactly:

- `ui.document.write`;
- `ui.events.read`;
- `menu.context.write`; and
- `session.close`.

The generated program has one compiled-in document, one compiled-in context
menu with one enabled semantic action, and one typed action-read loop. It does
not provide a point, native handle, command ID, shortcut, selection, link,
target, callback, event subscription, configuration loader, network access,
package identity, or signing behavior. The host retains all local trigger,
popup, placement, native lifetime, process, pipe, and shutdown facts.

## Consequences

- Developers can visibly exercise the complete first-party context-menu path
  without a browser runtime, Node.js, raw protocol JSON, or third-party UI
  library.
- Existing templates retain their smaller fixed capability sets.
- The generated-child integration test proves the private pipe, model
  replacement, revision-bound context action, and self-close without trying to
  synthesize native input. The real User32 right-click remains a manual gate.

## Revisit conditions

Revisit before adding a keyboard route, a secondary view, caller-provided menu
content, target/selection/link data, coordinates, shortcuts, callback, native
handle, dynamic model, richer popup feature, non-Windows adapter, product
identity, packaging, or signing. Each changes authority, privacy, or lifecycle
and needs its own contract, threat-model update, and verification.
