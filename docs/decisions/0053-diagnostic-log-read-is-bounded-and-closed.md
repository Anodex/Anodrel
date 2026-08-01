# Decision 0053: Diagnostic log reads stay bounded and closed

**Status:** Accepted

**Date:** 2026-08-01

## Context

The existing host `LogBook` already retains only a closed catalogue of
display-safe startup events. Its useful diagnostics are currently visible only
through the Startup Lab, despite the authenticated protocol already requiring
the `diagnostics.read` capability for `platform.health`. Exposing a general log
reader would risk turning host diagnostics into a path, error, request, or
application-data exfiltration channel.

## Decision

Protocol 1.11 adds `diagnostics.entries.read`. It accepts exactly `{}`, requires
the existing host-issued `diagnostics.read` grant, and returns at most the 64
retained records as `{ "entries": [...] }`. Each record has exactly the
existing display-safe fields: a canonical decimal `sequence`, the fixed
`"info"` level, and the closed catalogue's fixed `component` and `event`
labels. It accepts no filter, cursor, time, source, path, text, format,
subscription, export, or acknowledgement field.

The core receives an explicit portable diagnostics service. A host that did not
supply it fails closed with `diagnostics.unavailable`; it does not synthesize
native detail. The service exposes no write operation, persistence, process
history, application data, arbitrary host error, or operating-system API.

## Consequences

- authenticated applications can inspect the same bounded safe facts used by
  the Startup Lab;
- least privilege remains one existing diagnostics grant rather than a new
  catch-all logging capability; and
- the portable closed catalogue remains the sole source of protocol strings.

The tradeoff is that an application cannot search, stream, clear, export, or
write diagnostics, and cannot obtain a timestamp or native failure explanation.
Those require separate data classification and retention decisions.

## Revisit conditions

Revisit before accepting a dynamic event field, adding persistence, filtering,
pagination, export, subscriptions, crash data, application logging, telemetry,
or any new event source.
