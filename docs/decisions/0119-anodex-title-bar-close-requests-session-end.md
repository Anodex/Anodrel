# Decision 0119: Anodex title-bar close requests its owned session end

**Status:** Accepted and implemented in the Anodex adapter.

**Date:** 2026-08-29

## Context

Anodex's existing Electron title-bar close control resolves the window from its
own renderer sender and calls that window's native close method. It accepts no
window identity or lifecycle option, and it cannot select another window.

Anodrel's existing capability-gated `session.close` operation has the same
current-session ownership boundary. It accepts exactly `{}` and reports only
that the host accepted a request to end that session. Decision 0036 explicitly
does not make that response proof that a native window was destroyed or a
product process exited.

Creating an Electron-shaped `window.close` operation merely to make this
adapter look familiar would add a second lifecycle contract without adding a
new safe authority.

## Decision

`@anodrel/anodex-adapter` exposes `requestAnodexTitleBarClose`. It accepts only
a client with the existing `closeSession()` method and returns its exact
`{ status: "accepted" }` result.

The adapter names the action a *request* deliberately. It asks the host to end
the authenticated session that owns the current title bar; it does not claim
that a window, a process, or every view has already closed. It carries no
window ID, native handle, confirmation rule, close reason, process behavior,
or cross-session route.

The platform core, protocol, capability set, and Windows lifecycle code remain
unchanged. The adapter has no Electron import and does not host Anodex's
renderer.

## Consequences

- Anodex can map its current title-bar close intent to an explicit existing
  Anodrel lifecycle boundary without weakening ownership.
- A later Electron adapter can preserve Anodex's renderer-facing `Promise<void>`
  surface while both implementations are compared behind their adapters.
- Product close confirmation, window-close completion, background behavior,
  restart, and process termination remain separate contracts and tests.

## Alternatives considered

**Add a general `window.close` operation.** This would duplicate `session.close`
and invite a targetable window-lifecycle surface. Refused.

**Report native close completion from `session.close`.** That would make a
request response depend on UI and product-process lifetime, beyond the bounded
acceptance contract in Decision 0036. Refused.

**Make the adapter silently resolve `void`.** It would discard the only truthful
outcome available to Anodrel callers. Refused.

## Revisit conditions

Revisit before adding a close reason, confirmation, completion event, native
window identity, secondary-view title-bar route, process behavior, packaging,
or a non-Windows product host. Each changes lifecycle authority or evidence and
needs its own decision and verification.
