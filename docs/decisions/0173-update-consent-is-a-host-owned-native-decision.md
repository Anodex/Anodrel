# Decision 0173: Update consent is a host-owned native decision

**Status:** Accepted

**Date:** 2026-09-01

## Context

The native updater can now discover a signed candidate and invoke the fixed
UAC handoff, but it must not download or elevate merely because a background
component found an update. Allowing an application to provide arbitrary prompt
content or to remember a suppression choice would let it manipulate a security
decision or create an automatic update route.

## Decision

Add one direct Windows consent adapter that consumes only a CMS-verified opaque
update offer. It shows a fixed `MessageBoxW` confirmation with the signed
candidate version, `Yes`/`No` buttons, and `No` selected by default. The title
and text are fixed Anodrel-owned strings. An approval returns the original
opaque offer; a decline changes no state.

The adapter is native-host only. It has no application protocol, prompt text,
owner, endpoint, path, release notes, preference, progress, cache, network,
UAC, process, or installation input. A host calls it only from an explicit UI
action and keeps later blocking update work off the UI thread.

## Consequences

- The consent decision is distinct from both signed discovery and Windows UAC.
- An accidental keyboard confirmation does not approve an update because `No`
  holds the initial focus.
- There is no auto-update, remembered choice, or application-controlled dialog.
- Progress, restart behavior, an owned update screen, and end-to-end signed
  acceptance remain separate work.

## Revisit conditions

Revisit for a branded dialog owned by an existing host window, accessibility
testing, release notes, localization, remembered settings, progress, restart,
automatic scheduling, a public protocol capability, another platform, or a
signed end-to-end acceptance run.
