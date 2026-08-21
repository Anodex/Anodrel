# Decision 0081: Native child clients reuse the private transport without a runtime

**Status:** Accepted

**Date:** 2026-08-21

## Context

The Windows host can launch a verified native child, deliver a one-use `ANBI`
invitation through its standard input, and serve an authenticated named-pipe
session. The development probes use a developer-supplied Node.js process, while
the product fixture carries its own small, private Kernel32 pipe client and
framed request loop.

Neither is a reusable native application foundation. The former means a simple
diagnostic depends on an external development runtime; the latter duplicates
protocol-sensitive code inside a deliberately non-reusable fixture. Copying
that client logic into every native application would multiply the places that
can accidentally log an invitation, construct a pipe name, hold a native
handle, or mis-handle a coalesced response.

Anodrel must not solve that by shipping a browser, JavaScript engine, Node.js,
or a third-party IPC library. The child needs only its private invitation, a
direct operating-system stream adapter, and the already-owned wire and JSON
codecs.

## Decision

Add two first-party native-client modules with a deliberately narrow seam.

`anodrel-client` is portable Rust and owns a synchronous, authenticated framed
conversation over a caller-supplied byte stream. It reads one
`BootstrapInvitation`, emits its exact authentication message as the first
frame, keeps a bounded `FrameDecoder` for coalesced responses, and sends one
documented protocol request at a time. Its failures are closed categories; they
do not retain or display endpoint names, tokens, response bodies, paths, or
native error strings.

`anodrel-windows-client` is the Windows adapter. It opens only the exact pipe
name held by a validated invitation, through the smallest direct Kernel32
surface needed for synchronous data I/O. It never constructs, enumerates,
creates, secures, or hosts a pipe. It owns its one handle through RAII and has
no User32, policy, bootstrap-delivery, or application capability authority.

The modules form this one-way graph:

~~~text
native application
        |
anodrel-client (bootstrap, wire, JSON; no OS APIs)
        |
anodrel-windows-client (invited Kernel32 pipe only)
        |
Windows named-pipe server created by the host
~~~

The first migration reuses the modules in the signed product fixture and adds
one compiled native development probe. That probe is still a diagnostic:
the developer explicitly selects its executable, the host does not verify its
identity, and it does not become a package format, installer, product launcher,
or general application runtime.

This is a client transport foundation, not a replacement for the documented
TypeScript SDK. It introduces no application capability, protocol operation,
background event channel, script execution, public application template, or
stable cross-language ABI. A typed public native SDK is a later contract once
the executable application boundary is ready to publish.

## Consequences

- Anodrel can prove its launched-client route using an executable built entirely
  from Anodrel crates, the Rust standard library, and direct Windows APIs.
- Bootstrap, framing, response buffering, and handle lifetime have one reusable
  implementation rather than diverging between native child programs.
- Application code still receives no native handle, pipe-name constructor,
  capability grant, policy record, or direct host service; host policy remains
  the authority for every request.
- The initial API is synchronous and one-conversation-at-a-time. Async I/O,
  concurrent requests, subscriptions, reconnect, retry after authentication,
  cancellation scheduling, and application logging remain explicitly absent.

## Revisit conditions

Revisit before publishing a native SDK package, adding a cross-language ABI,
accepting an application-selected transport endpoint, adding asynchronous or
multi-request use, or supporting a non-Windows adapter. Each changes either
the public compatibility surface or the bootstrap and lifetime threat model.
