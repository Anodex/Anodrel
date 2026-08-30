# 0123 — Linux child bootstrap stays distinct from Windows

- Status: Accepted
- Date: 2026-08-30

## Context

Decision 0122 established one Linux-only, same-UID authenticated abstract Unix
socket. It intentionally did not define a child launch or invitation-delivery
channel. The existing Windows ANBI record validates a Windows named-pipe name;
expanding or reinterpreting it would weaken that platform-specific boundary.

A compiled Linux child needs one way to receive its host-created endpoint name,
session ID, and authentication token without placing them in an argument,
environment variable, file, diagnostic, or durable log. The portable client
also needs to authenticate an already-open stream without becoming a generic
endpoint or child-process API.

## Decision

Linux uses a separate EOF-delimited ANLI version 1 bootstrap record on private
child standard input. Its exact payload carries only:

- kind: linux.bootstrap.invitation;
- an endpoint name matching anodrel.v1. plus 64 lowercase hexadecimal
  characters;
- protocol version 1.0;
- a host-created session ID; and
- a 64-lowercase-hexadecimal-character authentication token.

The codec rejects unknown, missing, duplicate, malformed, oversized, truncated,
or trailing input. Its token is redacted from diagnostics and cleared on drop.
The Linux socket adapter may convert only its host-created invitation into this
record. The child adapter opens only the validated abstract endpoint from that
record; it has no constructor for a pathname socket, TCP address, arbitrary
endpoint, listener, or endpoint discovery.

The portable client receives one narrow authentication-invitation trait. It
builds the existing first session.authenticate frame and still owns ordered
request/response framing. Platform codecs implement that trait without exposing
a raw token getter. The current Windows ANBI call pattern remains
source-compatible and its format remains unchanged.

The first Linux child is a fixed development health probe. An integration test
starts a real Linux server, writes one ANLI record to that child’s standard
input, confirms authentication and platform.health, and observes a clean exit.
It is a transport test only: it does not supply a general launcher, executable
identity policy, a Linux window, package support, installation, or updates.

## Consequences

- Linux gains a real cross-process invited-client proof while retaining its
  separate operating-system and bootstrap contract.
- The implementation uses Anodrel crates, Rust standard Unix-socket support,
  Linux random bytes, and Linux peer credentials only.
- ANBI remains a strict Windows format; no Windows client, template, or product
  fixture needs to change.
- A future Linux launcher must separately define process identity, child
  lifetime, standard-handle inheritance, and desktop policy before it can be
  called a product launch.

## Alternatives considered

**Extend ANBI.** It would turn a Windows pipe-name validation rule into a
platform-neutral endpoint field. Refused.

**Pass secrets by arguments, environment variables, or a temporary file.**
Those channels are commonly observable or durable. Refused.

**Expose a generic Unix-socket client.** It would make endpoint selection a
client concern and blur the host-issued invitation boundary. Refused.

**Implement a Linux launcher now.** The project has not yet defined Linux
executable identity, window ownership, packaging, or update policy. Refused.

## Revisit conditions

Revisit before adding Linux process launch, a reusable application facade,
executable or package validation, a Linux desktop host, multiple child
sessions, non-standard-input invitation delivery, cross-user transport, or
macOS support.
