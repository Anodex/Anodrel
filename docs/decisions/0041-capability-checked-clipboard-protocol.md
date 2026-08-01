# Decision 0041: Clipboard protocol operations are separate and bounded

**Status:** Accepted

**Date:** 2026-08-01

## Context

The direct Windows clipboard adapter and portable text values establish safe
local behavior, but an application needs an authenticated way to request copy
and paste. A single combined grant, raw clipboard payload, or an unbounded
string would weaken the host's capability and transport boundaries.

## Decision

Protocol 1.5 adds `clipboard.read` and `clipboard.write` with separate
host-issued capabilities of the same names. Read accepts exactly `{}` and
returns either bounded text or `no_text`; write accepts exactly one UTF-8 text
field. Each protocol string is capped at 24 KiB, below the portable 64 KiB
limit, so one request always leaves bounded frame space. Native service errors
map to stable safe protocol codes and no clipboard data enters diagnostics.

The portable service interface is injected by the native host. The protocol,
core, SDK, mock host, and transport remain unaware of Windows APIs, owner
handles, raw formats, or native error values.

## Consequences

- applications can receive copy and paste authority independently;
- every live operation checks host policy before touching the clipboard;
- hosts can provide direct platform adapters without coupling the protocol to
  a particular operating system; and
- rich clipboard formats, history, subscriptions, consent, and automatic retry
  remain out of scope.

## Revisit conditions

Revisit before changing string limits, adding clipboard formats, adding events
or polling, adding consent, attaching clipboard access to a window selector,
or supporting another operating system.
