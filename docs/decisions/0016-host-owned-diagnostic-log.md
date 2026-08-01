# Decision 0016: Keep the first diagnostic log typed and host-owned

**Status:** Accepted

**Date:** 2026-07-31

## Context

The Startup Lab shows an **Open Logs** tile, but a conventional logging API
would immediately create a sensitive data boundary. Unstructured application
messages and native errors often contain paths, request data, credentials,
pipe invitations, and other values that cannot safely reach a diagnostics
surface or a future persistent log.

The platform needs a visible log diagnostic now, without pretending that a
general application logging service or crash-reporting pipeline is secure.

## Decision

`anodrel-diagnostics` owns a fixed-capacity in-memory `LogBook`. Its entries
are a closed typed event catalogue: sequence, severity, component, and
human-readable event text all originate in Anodrel code. The API accepts an
event enum, never a caller-provided message or structured payload.

The Windows Startup Lab records only its completed preflight checks and opens a
host-owned document for the **Open Logs** tile. The log is process-local,
non-persistent, non-exportable, and has no protocol, SDK, application, or
filesystem interface.

## Consequences

Positive:

- the tile becomes a truthful linked diagnostic rather than a placeholder;
- unsafe diagnostic data is excluded by the API shape rather than redacted
  after the fact;
- the portable log module is testable without Windows or a UI;
- no new application capability or operating-system authority is introduced.

Tradeoffs:

- the log cannot explain arbitrary failures or accept application messages;
- records disappear at process exit and cannot yet aid post-crash diagnosis;
- future durable or application-visible logging requires a separate design.

## Revisit conditions

Revisit before accepting any dynamic text or value, persisting/exporting a
record, reporting crashes, adding telemetry, or exposing a log through the
protocol. Each case needs explicit data classification, redaction rules,
capability semantics, retention behavior, and new threat-model coverage.
