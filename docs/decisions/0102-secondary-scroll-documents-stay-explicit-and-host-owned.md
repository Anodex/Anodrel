# Decision 0102: Secondary scroll documents stay explicit and host-owned

**Status:** Accepted

**Date:** 2026-08-21

## Context

Protocol 1.25 introduced bounded session-owned secondary views with an exact
version-1 document path. Protocol 1.26 later added explicit version-3 opening
and replacement for visible live-status documents. A primary view already has
an explicit version-2 replacement operation for the host-owned `scroll` node,
and the portable session-window group already contains isolated v2 replacement
logic. Secondary-view opening and replacement still reject v2 deliberately.

Requiring a scroll-only secondary view to use version 3 would couple ordinary
scrolling to a later live-status format it does not need. Conversely, silently
widening Protocol 1.25's version-1 operation would violate both the exact
document-format boundary and Decision 0092's requirement for a new decision
before opening a second document format.

## Decision

Protocol 1.27 adds two exact operations:

| Operation | Exact payload | Exact success result | Required grants |
| --- | --- | --- | --- |
| `window.open.v2` | `{ "title": string, "document": string }` | `{ "windowId": string }` | `window.open`, `ui.document.write` |
| `ui.document.replace.window.v2` | `{ "windowId": string, "document": string }` | `{ "revision": string }` | `ui.document.write` |

Both accept exactly one bounded `anodrel.ui.document.v2` document. The new
opening operation creates only a secondary view. The replacement operation may
address `main` or one current session-owned secondary identity, exactly as the
version-1 and version-3 targeted operations do. Version 1, 2, and 3 operation
names retain their existing exact decoders and protocol-minimum versions.

The host keeps one independent `UiScrollState` per view. Local wheel, keyboard,
pointer-scrollbar, UI Automation ScrollPattern, and ScrollItem behavior remain
host-owned and revalidate that view's current layout. They add no document
field, position readback, event, callback, focus route, native handle, or
application result. A v2 document cannot contain a status node, so it never
creates a live-region event.

The existing `ui.events.read.window` operation and its per-view revision-bound
semantic actions are unchanged. No capability, installed-record field,
bootstrap value, native creation setting, geometry, title authority, lifecycle
event, or cross-window service is added.

## Consequences

- A scroll-only secondary view can select the smallest document contract that
  represents its visible UI.
- Each document format remains explicit at protocol and decoder boundaries;
  older clients and hosts fail closed rather than guessing a format.
- The portable group, the authenticated core, the typed Rust client, the
  TypeScript SDK/mock, and the direct Windows host share the same revision and
  ownership model instead of creating a parallel scroll-specific window path.
- Verification must prove both a v2 secondary's initial snapshot and targeted
  replacement, then prove that v1 and pre-1.27 requests remain rejected.

## Revisit conditions

Revisit before adding a v2 status, scroll position or scroll event protocol
data, application-selected scrollbar behavior, nested-scroll targeting,
secondary native services, geometry, focus readback, lifecycle events,
additional document formats, production launch changes, or another operating
system adapter. Each changes an established authority or compatibility
boundary.
