# Decision 0034: Windows UI Session Lab consumes one bounded mailbox

**Status:** Accepted

**Date:** 2026-08-01

## Context

The authenticated transport can now publish a bounded UI document snapshot, but
that alone does not prove a host window uses the correct revision or remains on
its own UI thread. Reusing the developer preview would erase the distinction
between an operator-selected file and an authenticated session.

## Decision

The direct Windows host adds a development-only UI Session Lab. It owns one
`UiDocumentMailbox`, displays a host-created waiting document at first, and
polls that mailbox with a low-frequency window timer. A newer snapshot replaces
the view's document and resets its local layout state; no snapshot can select
another window. The pipe worker never calls User32 or the renderer.

The existing private-bootstrap development sample can be launched in this lab
with its explicit Node executable and sends one strict UI document after its
health check. This does not establish product executable trust, installed
policy provisioning, package content hosting, a public application window API,
or a general process launcher.

The Session Lab does not enable pointer, keyboard, accessibility, or semantic
action delivery for session documents. It only demonstrates authenticated visual
replacement. A visible action remains visual data until a separate event
transport contract exists.

## Consequences

- Anodrel can manually exercise a full authenticated document-to-native-window
  route without a webview or third-party runtime;
- the UI thread consumes only the mailbox it was explicitly given; and
- application interaction and production lifecycle remain deliberately out of
  scope.

## Revisit conditions

Revisit before enabling session-document input, exposing the lab as a product
window, subscribing one window to multiple sessions, adding a wake message, or
binding it to a verified installed application launch.
