# Decision 0043: External link protocol access is capability-checked

**Status:** Accepted

**Date:** 2026-08-01

## Context

The direct Windows external-link adapter can safely hand off a validated HTTPS
address, but application content must not invoke it merely by constructing a
protocol request. The capability, string limit, failure behavior, and service
injection boundary must be explicit.

## Decision

Protocol 1.6 adds `external.open`. It accepts exactly one URL string capped at
2 KiB and parses it using the portable `ExternalLink` type before any native
call. It requires one host-issued `external.open` grant. The core calls an
injected portable service immediately after that check and returns either
`{ "status": "opened" }` or the safe `external.unavailable` error.

The protocol, core, SDK, mock, and transport do not know Windows APIs, shell
verbs, browser names, or handler statuses. The direct Windows adapter is one
service implementation, not part of the protocol surface.

## Consequences

- applications can ask for a constrained user-browser handoff without shell
  strings or arbitrary URI schemes;
- the installed machine policy can grant link opening independently from all
  other platform authority; and
- confirmation, custom schemes, browser selection, callbacks, and history
  remain separate decisions.

## Revisit conditions

Revisit before changing the URL limit or accepted syntax, adding a scheme,
adding confirmation, returning handler detail, opening a native browser view,
or supporting another operating system.
