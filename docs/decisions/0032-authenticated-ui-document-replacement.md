# Decision 0032: UI document replacement is capability-checked and session-bound

**Status:** Accepted

**Date:** 2026-08-01

## Context

Anodrel's UI document codec and revision state establish safe portable data and
stale-event protection, but neither accepts an application request. A direct
connection from arbitrary protocol data to a native view would bypass the
platform's capability and session boundaries. The 64 KiB wire limit also means
the format's maximum 64 KiB raw document cannot safely be embedded in every
JSON request.

## Decision

Protocol 1.1 adds `ui.document.replace`. It is available only to an already
authenticated session with the host-issued `ui.document.write` capability. The
operation supplies one strict `anodrel.ui.document.v1` document string capped
at 24 KiB, which leaves bounded space for the enclosing 64 KiB wire request.
The host validates it through `anodrel-ui-session` and returns only the
resulting canonical revision string.

One authenticated transport owns one document session. Failed validation
leaves its prior document and revision unchanged. The operation cannot select a
window, read a document, submit a patch, attach a renderer, trigger a callback,
or deliver an input event. It is deliberately independent of the Windows
developer preview, which remains an operator-only local tool.

## Consequences

- application UI data now has an explicit authenticated, capability-checked
  entry point without acquiring native authority;
- a future native view can render a current immutable session document and
  reject stale events using its revision; and
- event delivery, window attachment, queue limits, cancellation, and lifecycle
  remain separate decisions rather than hidden in the document operation.

## Revisit conditions

Revisit before increasing the embedded-document quota, adding patches,
readback, multiple views, document persistence, native window attachment, or
application action-event delivery.
