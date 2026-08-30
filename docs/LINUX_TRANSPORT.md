# Linux local transport

**Status:** First Linux-native transport foundation. Its strict invited child
transport proof is implemented, but it is not a Linux desktop host, general
launcher, installer, package format, or application SDK.

`anodrel-linux-pipe` lets a future Linux host compose the existing portable
Anodrel frame codec and authenticated session engine over one direct Unix-domain
stream. It adds no protocol operation, capability, service, or runtime
dependency.

## Endpoint and authentication

The host creates one abstract-namespace `AF_UNIX` endpoint named:

~~~text
anodrel.v1.<64 random lowercase hexadecimal characters>
~~~

It has no filesystem path, directory, temporary file, cleanup routine, or TCP
port. The random suffix comes from 32 bytes read from `/dev/urandom` and only
selects an endpoint; it is not authentication.

The adapter accepts exactly one stream. Before it accepts any framed message,
it reads Linux `SO_PEERCRED` and requires the peer effective UID to match the
host effective UID. The first complete Anodrel frame must still be the existing
private `session.authenticate` control with a second, independently generated
32-byte token. The token is never exposed through a getter, written to a file,
logged, placed in an argument or environment variable, or sent in a protocol
response.

Once both checks pass, the existing `anodrel-transport` session handles frames,
capabilities, cancellation, and responses exactly as it does on Windows. A
failed UID check, authentication failure, malformed frame, or I/O error closes
the endpoint without additional native detail.

## Lifecycle

`LinuxPipeServer::serve_one` performs blocking-style stream work and belongs on
a dedicated host worker, never a future Linux UI thread. It accepts one peer,
then drops the listener's ability to accept a second client. The host-only stop
signal has no protocol representation: it wakes a pending accept with a local
connection and uses a short read timeout while a peer is connected. A stop,
peer disconnect, or session failure ends the worker cleanly.

The invitation clears its token buffer on drop and redacts it from `Debug`.
The abstract endpoint disappears when the listener is dropped.

## Explicitly absent

- a reusable Linux child-process launcher or executable identity policy;
- a Linux native window, renderer blit, dialogs, clipboard, logging,
  notification, credential, network, or policy adapter;
- a filesystem socket, TCP endpoint, multiple-client server, or cross-user
  connection;
- a public application client or public endpoint discovery;
- macOS support.

ANLI now supplies one distinct, child-standard-input invitation for the fixed
Linux health probe. The Windows ANBI record remains Windows-specific because it
validates a Windows named-pipe name. A reusable Linux launcher still needs its
own documented process, identity, lifecycle, and desktop policy boundary.

## Verification

Run the Linux adapter tests from an Ubuntu environment with a current Rust
toolchain:

~~~powershell
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-linux-pipe'
~~~

The tests run a real abstract Unix socket and prove successful authenticated
health, rejected invalid authentication, same-process stop before accept, and
stop of a connected worker. The separate compiled-child test is documented in
LINUX_NATIVE_CLIENT.md. None claim a visible Linux desktop surface.

See [Decision 0122](decisions/0122-linux-transport-uses-an-authenticated-abstract-unix-socket.md),
[Decision 0123](decisions/0123-linux-child-bootstrap-stays-distinct-from-windows.md),
and [the transport contract](TRANSPORT.md).
