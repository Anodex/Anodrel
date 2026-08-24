# Anodrel Windows native SDK

**Status:** Contract accepted; implementation follows Decision 0104.

## Purpose

`anodrel-windows-ui-sdk` is the stable in-repository Windows entry point for a
compiled native Anodrel development application. It owns private invited-session
setup and exposes existing typed UI-session methods without exposing transport
or operating-system authority.

It is not a product package format, launcher, installer, identity system,
update client, cross-platform runtime, or registry-published package.

## Entry point

An application creates one session by calling
`WindowsUiSession::connect_from_stdin`. The SDK reads one `ANBI` invitation from
standard input, opens only its exact invited Windows pipe, authenticates before
application requests, and drops the invitation after authentication. It exposes
no constructor that accepts a pipe name, stream, token, capability list, or
native handle.

Connection errors are closed categories. They never include bootstrap bytes,
pipe names, tokens, raw Windows errors, raw host responses, or host diagnostics.

## Typed session surface

The facade preserves the documented typed operations from `anodrel-ui-client`:
strict v1/v2/v3 document replacement, bounded semantic-event reads,
whole-surface field snapshots, complete menu replacement, opaque secondary-view
operations, and group close. Every method uses its minimum documented protocol
version internally; applications cannot choose an arbitrary operation or
protocol version.

Whether a method succeeds still depends on the host-issued grant. The SDK does
not declare, request, inspect, or broaden capabilities.

## Compatibility

The first surface is version `0.1.0` inside this repository. Additive changes
need public documentation and generated-template compatibility coverage. A
removal or incompatible type change requires a new decision and a new `0.2`
minor line. Registry publication is intentionally separate work.

The generated UI, menu, form, live-status, multi-window, and scroll-window
projects are the real consumers. Their isolated release builds and authenticated
Windows-pipe sessions prove that the SDK has no hidden host-source dependency.

See Decision 0104 and `docs/NATIVE_CLIENT.md` for the lower-level private
transport contract.
