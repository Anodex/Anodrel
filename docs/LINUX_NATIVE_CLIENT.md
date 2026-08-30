# Linux native child client

**Status:** Implemented development transport proof. This is not a Linux
desktop host, generic launcher, public application SDK, package format, or
product executable path.

## Purpose

The Linux child path joins the first direct Linux transport adapter to one
compiled, fixed-purpose health probe:

~~~text
host creates a Linux abstract socket
  -> host writes one ANLI invitation to private child standard input
  -> child opens only that invited abstract socket
  -> child sends session.authenticate as its first ANDR frame
  -> host confirms authentication
  -> child requests platform.health and exits
~~~

The probe accepts no arguments, configuration, file, environment, network
input, UI document, capability choice, or application content. It returns only
a fixed exit stage and never prints an invitation, endpoint, token, or native
failure.

## ANLI bootstrap v1

ANLI is Linux-specific. It uses the same bounded outer record shape as the
Windows bootstrap but it is a distinct format and must never be parsed as ANBI.

~~~text
0                   4                   8                  12
+-------------------+-------------------+-------------------+
| magic: "ANLI"      | major: u16 LE     | minor: u16 LE     |
+-------------------+-------------------+-------------------+
| payload length: u32 LE                                  |
+----------------------------------------------------------+
| UTF-8 JSON payload (at most 2,048 bytes)                 |
+----------------------------------------------------------+
~~~

Version 1.0 accepts exactly these fields:

~~~json
{
  "kind": "linux.bootstrap.invitation",
  "endpointName": "anodrel.v1.<64 random lowercase hexadecimal characters>",
  "protocolVersion": { "major": 1, "minor": 0 },
  "sessionId": "host-created opaque ID",
  "token": "64 lowercase hexadecimal characters"
}
~~~

The child reads one complete record through standard-input end-of-file. The
codec rejects every other field set, duplicate JSON key, malformed value,
oversized frame, truncation, and trailing byte. The token has no accessor,
never appears in Debug, and is cleared when its invitation is dropped.

## Boundaries

| Component | Owns | Does not own |
| --- | --- | --- |
| anodrel-linux-client | ANLI validation, the exact invited abstract socket, and token redaction | a listener, filesystem or TCP socket, endpoint discovery, child launch, policy, UI, or retries |
| anodrel-client | authentication-first framing and one ordered request/response exchange | Linux APIs, endpoint selection, process ownership, or capability policy |
| Linux host | endpoint creation, same-UID peer verification, policy, and child lifetime | child application behaviour |

The endpoint and token remain host-issued. The endpoint name alone is not
authentication: the server additionally verifies SO_PEERCRED against its
effective UID, then validates the independent token in the first transport
frame.

## Verification

From Ubuntu or WSL with Rust installed:

~~~powershell
wsl -- bash -lc 'source "$HOME/.cargo/env" && cd "/mnt/c/Users/Owner/Desktop/Platform X/native" && CARGO_TARGET_DIR=/tmp/anodrel-linux-target cargo test -p anodrel-native-linux-client-sample'
~~~

The integration test starts a real abstract socket and a separate compiled
child process. A second test proves the child refuses to search for an endpoint
when standard input has no invitation. Passing tests do not demonstrate a
visible Linux surface, installation, signing, packaging, updates, or general
process launch.

See [Decision 0123](decisions/0123-linux-child-bootstrap-stays-distinct-from-windows.md),
[Linux transport](LINUX_TRANSPORT.md), and [the native transport contract](TRANSPORT.md).
