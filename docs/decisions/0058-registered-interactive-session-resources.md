# Decision 0058: Keep registered interactive-session resources host-owned

**Status:** Accepted

**Date:** 2026-08-08

## Context

The registered Windows-session adapter can now derive policy and identity-bound
services from one machine-validated installed record. The native UI session
foundation already has separate bounded document, semantic-input, close, and
file-dialog resources, but it is used only by a development diagnostic.

Passing those resources independently through a future launch path would make
it easy to cross sessions: a window could accidentally poll one application's
mailbox while a pipe worker serves another. Letting the application construct
or select them would undermine the host ownership boundary.

## Decision

Add `RegisteredUiSession` in the Windows registered-session adapter. The host
selects the installed application ID, local host name, and session ID; only
after policy and package validation does the adapter create one pipe endpoint,
one sensitive invitation, and one `RegisteredSessionUi` resource group.

The group contains exactly one of each host-created resource:

- latest-document mailbox;
- revision-bound semantic-input mailbox;
- coalescing session-close signal;
- one UI-thread file-dialog mailbox; and
- one retained selected-file text service.

The same resources are attached to the transport before authentication and are
returned only as a group to host code. The group has no native window handle,
application-selected title, launch authority, process handle, or privileged
operation. The Windows host has a separate host-only authenticated-window entry
point that accepts this group; it remains unavailable to applications and is
not wired to Startup Lab until a signed provisioned application can pass the
locked launch service.

## Consequences

Positive:

- transport, native UI, close handling, and UI-thread file routing share one
  session boundary;
- a product-launch coordinator can compose a verified child and its window
  without reimplementing the diagnostic path;
- applications cannot create windows or swap resources across sessions.

Tradeoffs:

- this is a host integration foundation, not an installed product launch;
- window lifetime must still be joined to the tracked child lifecycle when an
  actual signed fixture and installation provisioning exist.

## Revisit conditions

Revisit when public multi-window policy, accessibility adapters, window
restoration, restart coordination, or concurrent product sessions require a
larger host lifecycle model.
