# Decision 0092: Session-owned multi-window views use opaque identities

**Status:** Accepted

**Date:** 2026-08-21

## Context

The direct Windows host already owns a correct private lifecycle for several
native top-level windows. It can dynamically open host documents from Startup
Lab while the normal User32 loop is running. An authenticated UI session,
however, still has one document state, one input queue, and one native view.

Exposing the private registry as a window API would be a security and
compatibility failure. A Win32 handle is native authority. A desktop position
and monitor reveal machine topology. An application-provided target would
invite cross-session routing. Reusing the primary document or input mailbox for
a second window would make revision collisions and event attribution
ambiguous.

## Decision

The first public multi-window feature is a bounded group of session-owned
logical views. The group starts with `main` and permits at most three secondary
views. The host assigns every secondary a canonical opaque `window-<n>`
identity, keeps the map to native windows private, and never reuses a closed
identity in the session.

Each view has its own `UiDocumentSession`, revision history, document mailbox,
and 32-item semantic input queue. The portable UI-session crate owns this
state before any protocol or Win32 bridge uses it. That keeps validation,
limits, identity parsing, and stale-event isolation testable without a desktop
or an operating-system API.

The reserved Protocol 1.25 surface separates four powers: `window.open` needs
both `window.open` and `ui.document.write`; `window.close` needs
`window.close`; view-targeted document replacement needs only
`ui.document.write`; and view-tagged event retrieval needs only
`ui.events.read`. Existing targetless window services retain their
primary-view-only meaning. No operation accepts a native handle, geometry,
monitor, cross-session target, or window enumeration selector.

A session-close request is group-wide. The eventual Windows adapter must create
and register a new native view before showing it, roll back both native and
logical state when creation fails, and retain tracked product-session lifetime
until the final session view leaves. A secondary view does not automatically
inherit privileged primary-only native bridges.

## Consequences

- Anodrel gets a real path to independent documents and semantic events across
  several app windows without a browser runtime or a framework window object.
- The first implementation can prove portable identity, independent revisions,
  bounded resources, and close semantics before it reaches User32 or the
  authenticated pipe.
- A caller can know only a logical identity it was issued in its own session;
  it cannot use that identity to learn host topology or affect another session.
- Supporting native window creation now entails a session-group lifetime model,
  not only another `CreateWindowExW` call.

## Revisit conditions

Revisit before adding a second document format to opening, an application
selected size or position, a modal/owner relationship, primary closure
behaviour, a lifecycle event, menu or dialog routing on secondary views,
restoration, a production-product session, or a non-Windows adapter. Each
changes the authority, observation, or lifetime boundary and needs a separate
decision.
