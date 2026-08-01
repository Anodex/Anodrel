# Decision 0040: Clipboard starts with bounded Unicode text

**Status:** Accepted

**Date:** 2026-08-01

## Context

Desktop applications need copy and paste, but the operating-system clipboard
can carry arbitrary native handles, rich formats, files, history, and data from
other applications. Exposing that ambient surface directly would undermine
Anodrel's explicit capability boundary.

## Decision

Anodrel's first clipboard service is a direct operating-system adapter for one
bounded UTF-8 text value. Its portable interface has explicit read and write
operations. The eventual protocol uses separate `clipboard.read` and
`clipboard.write` grants. The Windows adapter accepts and produces only
`CF_UNICODETEXT`, owns no clipboard data after a call, and returns stable safe
failure categories rather than native errors.

## Consequences

- applications can gain common copy/paste behavior without raw native handles;
- clipboard data remains unlogged and bounded before it crosses a host boundary;
- rich formats, clipboard events, images, files, and history need separate
  contracts and security reviews.

## Revisit conditions

Revisit before adding new clipboard formats, polling/retry behavior, change
notifications, an application-facing protocol operation, consent, persistence,
or non-Windows adapters.
