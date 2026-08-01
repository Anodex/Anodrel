# Decision 0011: Windows host claims one instance per validated application identity

**Status:** Accepted

**Date:** 2026-07-31

## Context

The direct Windows host can validate a no-script package and create a native
window, but independent invocations can create competing windows for the same
validated application identity. That leaves basic host lifecycle behavior below
the Phase 2 acceptance bar and makes the development surface unlike a desktop
application host.

Anodrel does not yet launch a verified executable or expose a public lifecycle
protocol. Passing arbitrary command-line data, URLs, or file paths from a
second process to a primary process would create an unauthenticated application
bridge before that security boundary exists.

## Decision

Add `anodrel-windows-instance`, a direct-Win32 adapter. It creates a
current-session `Local\\` mutex and readiness event from a validated
application ID plus a host-controlled scope. The primary holds both handles
until its native window closes. A second host invocation waits no longer than one
second for readiness and broadcasts a matching registered User32 message that
asks the primary window to restore and request foreground activation.

The `application` and `startup-lab` scopes are deliberately distinct: the
application text surface is a candidate application lifecycle, while Startup
Lab is an independently runnable host diagnostic. No input, identity, data,
secret, or request payload is forwarded to the primary process.

## Consequences

Positive:

- a validated application text surface has a real, bounded first-instance
  lifecycle on Windows;
- the behavior remains in a small adapter that future macOS and Linux hosts can
  replace behind the same lifecycle contract;
- no external runtime, browser integration, or public IPC surface is added.

Tradeoffs:

- focus activation is subject to Windows foreground policy and is intentionally
  best effort;
- the feature coordinates only a native window, not a product application
  executable;
- a same-session process can cause a local denial of availability, but cannot
  acquire platform authority or receive data through this mechanism.

## Revisit conditions

Replace this narrow path when Anodrel defines verified executable launch and a
versioned second-instance lifecycle protocol. Any future command forwarding
must authenticate the target application identity, define bounded payloads, and
be tested for startup races and untrusted input.
