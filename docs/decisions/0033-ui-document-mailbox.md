# Decision 0033: UI document delivery coalesces in one session mailbox

**Status:** Accepted

**Date:** 2026-08-01

## Context

`ui.document.replace` is processed on an authenticated transport session, but
a native window must run on its operating system's UI thread. Forwarding every
accepted replacement through an unbounded channel would let a fast application
consume memory and delay a visual surface. Calling a renderer or window API
from the pipe worker would cross the host's thread boundary unsafely.

## Decision

Anodrel provides a portable `UiDocumentMailbox` for one session. It stores at
most one immutable snapshot: a validated `UiDocument` and its revision. A newer
published revision replaces an older pending revision; an older revision cannot
replace a newer one. The consuming host takes and clears the one pending value.

The transport publishes only after the core has atomically accepted a document.
The mailbox does not signal a window, perform I/O, invoke a callback, queue
semantic actions, or identify an application. A host explicitly owns mailbox
creation, window lifetime, and notification or polling strategy.

## Consequences

- the pipe worker never has to wait for a renderer or UI message loop;
- memory for pending visual delivery remains bounded by one validated document;
- intermediate visual states may be coalesced, while protocol responses retain
  their own request-level success result; and
- attaching a mailbox to a window remains a separate host decision.

## Revisit conditions

Revisit before adding wake notifications, multiple consumers, retained history,
semantic event delivery, a window attachment API, or a document persistence
mechanism.
