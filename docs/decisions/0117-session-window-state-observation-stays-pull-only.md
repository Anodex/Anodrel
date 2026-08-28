# Decision 0117: Session window state observation stays pull-only

**Status:** Accepted

**Date:** 2026-08-27

## Context

Decision 0072 deliberately made `window.state.set` a closed, write-only
presentation command. An application can minimise, maximise, or restore only
the native window that its authenticated session already owns. It cannot learn
the result, because issuing a native command is not the same as observing the
desktop.

The first Anodex migration seam is its custom title bar. Its current Electron
adapter reads whether its own main window is maximised when the title bar opens
so it can choose the correct accessible label and glyph. Replacing that adapter
with only an optimistic local flag would drift after a person uses ordinary
Windows controls. Exposing a native handle, a window list, placement, or a
generic event subscription would solve a much larger problem and break the
session-owned boundary.

## Decision

Protocol 1.30 adds a separate `window.state.read` capability and one exact
operation, `window.state.get`. The payload is exactly `{}`. On success, it
returns one current closed state:

- `minimized`;
- `maximized`; or
- `restored`.

The host resolves the native window only from the authenticated session and
asks its owning UI thread for one snapshot. It accepts no target, handle,
window identity, geometry, monitor, display mode, focus, visibility, z-order,
fullscreen flag, timestamp, sequence number, native command, or query option.

The operation is a bounded pull, not a subscription. It creates no callback,
event, background receiver, listener test, change history, or delivery
guarantee. A returned state can be stale as soon as the response leaves the
host. `window.state.set` remains unchanged and still reports acceptance only.

Installed application record version 1.17 is the first record that may name
`window.state.read`; it is a strict superset of version 1.16. Older records
that name it are invalid. The normal five-second, one-request session-to-UI
thread bridge applies. An absent, expired, or failed UI surface returns
`window.unavailable`; another read already waiting returns `window.busy`.

## Consequences

- An application can accurately initialise a title bar for its own window
  without gaining any cross-window or geometry observation capability.
- The Anodex migration can begin with a truthful desktop-service adapter rather
  than an Electron-shaped compatibility shim inside the Anodrel core.
- The Windows adapter remains the sole owner of native state queries. Portable
  code sees only the existing three-value `WindowState` enum.
- This does not make Anodrel a React, webview, or browser-runtime host, and it
  does not claim that Anodex can yet run on Anodrel.

## Alternatives considered

**Return state from `window.state.set`.** This would silently change the
write-only contract of Decision 0072 and would still not supply an initial
snapshot. Refused.

**Expose Electron-style maximise events.** A subscription introduces lifetime,
delivery, rate, reconnect, and event-order policy. The first migration need is
an initial snapshot, so events remain separate work. Refused.

**Expose a general native window object.** It would make handles, topology, and
future native commands ambient application authority. Refused.

## Revisit conditions

Revisit before adding change events, subscriptions, another state value,
fullscreen observation, geometry, focus, window targeting, secondary-view
observation, a non-Windows adapter, packaging, or production identity. Each
changes the authority, privacy, compatibility, or lifecycle boundary and needs
its own decision and threat-model entry.
