# Decision 0226: Window-controls UI Automation probing remains host-only

**Status:** Accepted

**Date:** 2026-09-19

## Context

Anodrel's targetless window-title, size, state, focus, and fullscreen controls
already have protocol, policy, and host tests. Their visible Windows effects
were otherwise proved only by a person. The existing UI Automation Invoke probe
proves one authenticated semantic action, but not that a request changed the
host's composed caption or reversible native presentation.

Giving an application window geometry, UI Automation selectors, or a test
result would violate the existing write-only control boundaries. A generic UI
Automation harness would also create a broad desktop-inspection and input tool.

## Decision

Add one development-only `--uia-window-controls-probe` route. It accepts one
development executable path. The standard suite supplies one compiled
first-party child, while the host finds only eight fixed semantic action IDs in
the document supplied through the existing session route. A private host worker
attaches only to the just-created session window through Windows UI Automation,
invokes the fixed actions, and compares only its own window name and visible
rectangles. This development route makes no code-identity claim for an
arbitrary executable path.

The worker confirms a host-composed title, changed rectangle after bounded
resize, maximise/restore, fullscreen/windowed restoration, and ordinary session
completion. It does not report an observation to the child, protocol, SDK, or
application. The focus step verifies only its existing acknowledgement path;
Windows still decides whether foreground attention succeeds.

## Consequences

- One real Windows acceptance route now covers bounded presentation changes
  without giving applications native readback or UI Automation access.
- The repeatable accessibility suite gains a seventh fixed direct-client probe.
- Manual checks remain required for minimisation, visible behaviour, scaling,
  position/z-order, multi-monitor fullscreen, Narrator, and Inspect.

## Revisit conditions

Revisit before accepting any application-selected child behavior, document,
action, selector, coordinate, title, window, monitor, geometry, focus result,
callback, or non-Windows equivalent.
