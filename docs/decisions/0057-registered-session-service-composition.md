# Decision 0057: Compose registered sessions from identity-bound native services

**Status:** Accepted

**Date:** 2026-08-08

## Context

The Windows registered-session adapter already converts a machine-selected,
validated installed application record into named-pipe session policy. Its
endpoint previously used the default unavailable service seams, leaving the
installed application identity disconnected from services that require it,
such as state storage and Credential Manager.

Passing a growing list of adapters through each core, transport, and pipe
constructor would make service ownership unclear and make accidental ambient
authority easier as new features arrive.

## Decision

Add a portable `HostServices` bundle. It starts with every service unavailable
and is consumed exactly once when the native host constructs a core session.
The transport and Windows pipe adapters accept this bundle without inspecting
or mutating its service contents.

`anodrel-windows-registered-session` now derives one bundle only after the
machine policy record and its package identity have validated. It attaches:

- current-process Unicode text clipboard access;
- validated HTTPS external-link handoff;
- one host-owned state store derived from the application identity; and
- one Credential Manager namespace derived from the same identity.

File dialogs, selection-scoped file reads, native UI document delivery, close
signals, and diagnostics remain unavailable in this registered launch path
until their public window lifecycle and host ownership rules are complete.
Capability grants remain an independent machine-policy decision: a granted
operation without an attached service returns its existing safe unavailable
result; no grant can create a service or broaden authority.

Installed record version 1.2 adds the already documented storage, credential,
and file-operation grant names. Version 1.1 remains limited to its original
grant set; this preserves exact versioned policy interpretation.

## Consequences

Positive:

- registered product sessions now use the same validated identity for policy,
  state isolation, and credentials;
- service construction is one explicit, small composition boundary rather than
  a chain of growing constructors;
- unimplemented native-window services still fail closed.

Tradeoffs:

- a provisioned signed record and product executable are still required before
  Startup Lab can invoke the locked launch service;
- public native window lifecycle remains the next gate for UI-bound services.

## Revisit conditions

Revisit when user consent, service-specific parameters, public window
lifecycle, multi-process service brokering, or non-Windows session adapters
need additional composition rules.
