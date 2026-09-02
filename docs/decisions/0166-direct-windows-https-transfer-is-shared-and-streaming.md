# Decision 0166: Direct Windows HTTPS transfer is shared and streaming

**Status:** Accepted

**Date:** 2026-09-01

## Context

Anodrel already owns a narrow direct WinHTTP text-fetch adapter. The owned
updater will need the same TLS and state-suppression posture but must transfer
installer images up to 576 MiB. Duplicating the raw WinHTTP bindings would let
security settings drift; buffering an installer in memory would be wasteful and
unnecessarily expose the process to large allocation pressure.

## Decision

Move the direct WinHTTP handle lifecycle into one internal shared transport.
It accepts only a caller-validated exact origin and absolute request target,
one bounded body limit, one expected-status rule, and a caller-owned streaming
chunk consumer. It creates a fresh no-proxy synchronous `GET`, retains all
existing fixed ten-second timeouts, disables cookies, redirects, automatic
authentication, and keep-alive, enables revocation checking, and returns only
closed failure categories.

The transport retains no body. The text adapter collects a 32 KiB UTF-8 body
above it; a later update adapter will stream a bounded binary image directly to
its private fresh file while hashing it. The shared transport is not a public
protocol service and cannot choose origins or request targets itself.

## Consequences

- Existing and future owned HTTPS paths use one audited FFI and RAII handle
  implementation rather than copy-pasted security settings.
- Large update images can remain bounded in memory by their fixed chunk size.
- Application-facing text-fetch authority and host-owned release retrieval stay
  distinct adapters with distinct parsing, size, and policy rules.

## Revisit conditions

Revisit before introducing redirects, proxies, credentials, cookies,
background/concurrent transfers, request bodies, headers, alternate TLS policy,
another operating-system adapter, streaming resume, or a public binary protocol
operation.
