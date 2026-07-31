# 0009: Deliver pipe invitations through a one-use inherited bootstrap handle

- Status: Accepted
- Date: 2026-07-31

## Context

The authenticated Windows named-pipe adapter creates a private pipe name,
session identifier, and authentication token. A launched application needs
those values before it can connect, but exposing them through command-line
arguments, environment variables, logs, or a predictable file would widen the
credential's visibility.

## Decision

The Windows host delivers one private invitation through an anonymous pipe
connected to the launched process's standard input. The direct Windows launcher
creates the pipe, clears inheritance on the parent write endpoint, and supplies
an explicit handle list to `CreateProcessW`. The child inherits only its
standard-input read endpoint and `NUL` standard-output/error handles.

The record is one bounded, versioned `ANBI` frame with a strict JSON payload.
The host writes it once and closes the writer. The child rejects malformed,
truncated, oversized, incompatible, or additional data. The sensitive payload
is never represented in the command line, environment, log, or durable file.

The launcher accepts a caller-supplied executable path and non-secret argument
list. It is not yet a product application trust policy, package verifier,
content host, or restart manager.

## Consequences

- Invitation delivery is independently testable and does not couple the pipe
  adapter to a UI toolkit or application framework.
- The direct Win32 process and handle code is isolated in a dedicated adapter.
- A child must opt into consuming the bootstrap stream before using its normal
  standard input for any other purpose.
- Future application launching must validate executable identity, control
  lifecycle and timeouts, and define rendered-content isolation before it can
  use this facility in production.

## Revisit when

Revisit this decision if a supported operating system cannot express a
one-use inherited child handle safely, if a future packaged runtime requires a
different private bootstrap primitive, or if application isolation requires a
brokered process model.
