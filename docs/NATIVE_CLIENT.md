# Anodrel native child client

**Status:** Implemented and tested: the portable `anodrel-client` core, direct
`anodrel-windows-client` adapter, stable `anodrel-windows-ui-sdk` facade,
migrated product fixture, and compiled native health and UI-session development
probes. The existing Node.js diagnostic remains separately useful for
development paths that exercise the broader service set.

## Purpose

`anodrel-client` is the first-party client half of Anodrel's existing private
child transport. It lets a compiled native child consume the invitation that a
host has already delivered, authenticate to that one host-created endpoint, and
exchange documented protocol messages.

It is intentionally smaller than an application runtime and smaller than a
public SDK. It does not render a surface, execute scripts, create windows,
select permissions, discover services, or launch another process. Its only job
is to move a bounded conversation across an already-authorized local stream.

## Boundaries

| Layer | Owns | Does not own |
| --- | --- | --- |
| `anodrel-client` | `ANBI` invitation consumption, authentication-first framing, one ordered request/response exchange, bounded queued response frames | Windows handles, pipe creation, policy, capabilities, process lifecycle, UI, retries after a session fails |
| `anodrel-windows-client` | Opening and closing the invitation's exact named pipe with direct Kernel32 data I/O | Pipe names, pipe security, server creation, bootstrap delivery, User32, or application policy |
| Native child | Its fixed application behaviour and safe exit status | Host identity verification, capability grants, native window ownership, or access to another session |
| Host | Invitation creation, authenticated session policy, window and process lifetimes | Application behaviour after the child receives a valid response |

The portable module must remain free of operating-system APIs and `unsafe`
code. All direct Windows FFI stays in the adapter, alongside the server-side
Windows adapters.

## Start-up sequence

~~~text
host starts a chosen child
  -> child reads one ANBI frame from standard input
  -> Windows adapter opens exactly invitation.pipeName
  -> client writes session.authenticate as its first ANDR frame
  -> host confirms session.authenticated
  -> child sends one documented request and reads its response
~~~

The invitation is sensitive. It may not appear in command arguments,
environment variables, files, logs, diagnostics, panic reports, standard
output, or standard error. The native child drops it immediately after
authentication. It cannot use a different pipe name or create an endpoint of
its own.

## Conversation rules

- `session.authenticate` is always the first outgoing frame.
- The client sends one request and reads its matching response before sending
  another. There is no background receiver, callback, subscription, or
  concurrent request map.
- The owned `FrameDecoder` enforces Wire v1's 64 KiB frame limit and four-frame
  receive-burst limit. A fragmented or coalesced pipe read is ordinary; a bad
  frame ends the client session.
- A client may request only the documented protocol version and operation its
  application was written to use. The host remains authoritative: an unknown,
  ungranted, malformed, or unsupported request is a safe host response, not
  client-side authority.
- Connection, framing, authentication, malformed response, and failed-status
  outcomes become stable client failure categories. They must not expose a
  token, endpoint, raw payload, Windows error, or host-owned detail.

## Windows adapter limits

The adapter requests only pipe data access, shares nothing, and opens the exact
UTF-16 name from the invitation. If the hosted endpoint is briefly busy, it may
perform the existing bounded wait; it does not reconnect after a successful
connection ends. `Drop` closes the one client handle exactly once.

The adapter performs blocking reads only on the child application thread. It
never runs on Anodrel's Windows UI thread; host UI responsiveness remains a host
requirement, not an application permission.

## Verification

Portable unit tests cover authentication ordering, fragmented and coalesced
responses, failure categories, and the bounded decoder. Windows adapter tests
cover UTF-16 conversion and absent-endpoint failure. The migrated product
fixture covers the joined lifetime: it uses these two modules through the real
child-only bootstrap channel and authenticated Windows pipe, then proves
document delivery, semantic input, session close, child exit, and server
cleanup. The compiled native health probe exercises bootstrap, authentication,
`platform.health`, and clean exit without Node.js. The separate compiled native
UI probe exercises the same private child route plus fixed document delivery,
one revision-bound semantic action, and `session.close` through a host-owned
Windows view. Both have real-pipe integration coverage.

No test records or prints invitation contents. The direct Windows adapter is
checked with workspace formatting, tests, linting, and the runnable development
diagnostic:

~~~powershell
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-client-sample
$clientPath = (Resolve-Path native/target/release/anodrel-native-client-sample.exe).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --native-sample-client $clientPath
~~~

It prints a safe success line and exits; it opens no window and requires no
Node.js process. A nonzero probe stage identifies only bootstrap, connection,
authentication, or health—not an invitation, pipe name, token, or Windows
error.

The compiled UI diagnostic uses the same owned modules but opens one temporary
host-controlled window. Build and run it with:

~~~powershell
cargo build --release --manifest-path native/Cargo.toml -p anodrel-native-ui-client-sample
$uiClientPath = (Resolve-Path native/target/release/anodrel-native-ui-client-sample.exe).Path
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --native-ui-sample-client $uiClientPath
~~~

Activate **Complete native UI diagnostic**. The child accepts only that action
at its own first document revision, then requests `session.close`; the host
closes that one window and prints a safe success line. This is a development
test, not a trusted launch, application template, or public native UI API.

## Compatibility

This contract adds no protocol version and no wire format. `ANBI` bootstrap v1
and `ANDR` wire v1 remain the existing host contracts in `docs/TRANSPORT.md`.
The lower-level native-client modules remain implementation crates rather than
the application SDK. Decision 0104 establishes `anodrel-windows-ui-sdk` as the
stable in-repository Windows application facade. It owns standard-input
invitation consumption, exact invited-pipe opening, and authentication before
returning the existing typed UI session. The generated templates consume only
that facade, and their isolated builds plus real pipe sessions are its
compatibility proof. See `docs/WINDOWS_NATIVE_SDK.md`.

The facade also carries the existing typed, targetless controls for the
authenticated session's own host window: title proposal, closed state,
foreground request, reversible fullscreen, and bounded logical client size.
Those calls retain their individual host-issued grants and provide no native
handle, window target, state or geometry readback, or cross-window route.

Decision 0105 exercises that complete control surface only through the separate
fixed-grant native window-controls template. It has no SDK method for choosing
its grant set, target, native handle, geometry, monitor, current state, focus
result, or title readback. See `docs/NATIVE_WINDOW_CONTROLS_TEMPLATE.md`.

Registry publication remains separate work. Decision 0094's existing bounded
multi-window methods return opaque secondary identities and tagged action
batches, but expose no native mapping or raw protocol surface. See
`docs/NATIVE_MULTI_WINDOW_TEMPLATE.md`.

Decision 0095 extends that preview facade only for the separate fixed-grant
native form template. Its `read_fields` method returns one strictly parsed,
whole-surface ordered snapshot after an explicit request; it has no selector,
live change route, or input-state metadata. See `docs/NATIVE_FORM_TEMPLATE.md`.

Decision 0101 uses the SDK's existing typed `replace_document_v3` method only in the
separate fixed-grant native live-status template. Its three fixed documents
demonstrate outbound visible status changes, not an accessibility callback,
listener check, or delivery API. See `docs/NATIVE_LIVE_STATUS_TEMPLATE.md`.

Decision 0102 adds the SDK's `open_window_v2` and
`replace_window_document_v2` methods for exact scroll documents in an existing
session-owned group. They share the existing window and document grants and
expose no scroll position, callback, native view, or automation route. See
`docs/MULTI_WINDOW.md`.
