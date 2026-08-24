# Decision 0104: Stable Windows native SDK keeps the invited session closed

**Status:** Accepted

**Date:** 2026-08-24

## Context

Anodrel's compiled native diagnostics and generated development templates
already prove the invited Windows-pipe route using first-party crates only.
They currently assemble three preview layers themselves: invitation reading and
authentication from `anodrel-client`, exact pipe opening from
`anodrel-windows-client`, and typed document/session calls from
`anodrel-ui-client`.

That assembly is safe, but it is not a stable application-facing surface. A
template must know which transport types to construct, and direct use of
`Client::request` makes a raw protocol operation appear adjacent to ordinary
typed application code. Publishing the raw transport as the SDK would let an
application choose an operation name, protocol version, and payload shape
outside the narrow typed boundary Anodrel has proved.

The platform needs a source-available Windows SDK before it can claim that a
native application need not know its host internals. It must not use that work
to add a capability, expand the invitation channel, select a pipe, or pretend
that development executables are installed products.

## Decision

Add `anodrel-windows-ui-sdk` as the stable, Windows-specific 0.1 application
facade over the existing invited session.

Its one constructor reads the private invitation from standard input, opens
only its exact Windows named pipe, authenticates immediately, consumes the
invitation, and returns one typed `WindowsUiSession`. Its public connection
errors are closed categories: bootstrap unavailable, invited endpoint
unavailable, or authentication unavailable. They carry no invitation, token,
pipe name, native error, or raw response.

`WindowsUiSession` exposes only the existing closed `anodrel-ui-client`
operations and typed result values: strict document replacement, bounded
semantic-event and field snapshot reads, complete menu replacement, existing
bounded secondary-view operations, and self/session-group close. It does not
expose `Client`, `WindowsClientStream`, a raw request method, a protocol-version
selector, endpoint input, capability input, native handle, background receiver,
retry, reconnect, subscription, or application identity.

The SDK is stable within the repository at version `0.1.x`: an additive method
requires documentation and compatibility coverage; removing or changing a
published method requires a new decision and a new `0.2` minor line. It is not
published to a registry, does not make a generated executable trusted, and
does not decide packaging, installation, signing, updates, a cross-language
ABI, or a non-Windows implementation.

Every generated native development template must consume this facade rather
than importing the three transport layers directly. Existing isolated build and
real invited-session tests become the SDK's compatibility proof.

## Consequences

- Native application source gains one documented entry point and has no reason
  to construct a protocol envelope or pipe client.
- Bootstrap and named-pipe limits remain inside first-party implementation
  crates, preserving host endpoint and capability authority.
- Existing template routes keep fixed grant sets; the facade has no API for
  widening them.
- A real generated-child session exercises bootstrap, authentication, typed
  requests, semantic input, and shutdown through this facade.

## Revisit conditions

Revisit before publishing to a registry, adding a generic stream constructor,
raw request API, asynchronous or concurrent use, reconnect or retry policy,
callbacks/subscriptions, an application-selected endpoint, a cross-language
ABI, product executable identity, packaging, installation, updates, or a
non-Windows adapter. Each would expand a compatibility or authority boundary.
