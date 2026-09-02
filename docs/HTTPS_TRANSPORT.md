# Direct Windows HTTPS transport

**Status:** Internal shared transport foundation. It is not an application
protocol operation and does not add network authority.

## Purpose

`anodrel-windows-http` owns the direct WinHTTP handle lifecycle for one
caller-authorized HTTPS `GET`. It is shared by the existing bounded text-fetch
adapter and future host-owned update retrieval so the two paths cannot drift in
their TLS, timeout, proxy, redirect, cookie, authentication, or cleanup rules.

The adapter accepts a validated `NetworkOrigin`, a validated absolute request
target, an expected-status policy, a fixed body limit, and a caller-owned chunk
consumer. It returns only the bounded numeric status or a closed failure
category. It never exposes native handles, headers, certificate details,
addresses, proxy details, redirects, timing, cookies, or credentials.

## Fixed request behavior

Each request creates a fresh direct no-proxy WinHTTP session and performs only
one synchronous secure `GET`. All resolve, connect, send, and receive phases
use fixed ten-second limits. The request disables cookies, redirects,
automatic authentication, and keep-alive, enables TLS certificate-revocation
checking, and sets no certificate-error-ignore flag.

The caller supplies one body limit. The transport reads at most that many bytes
in fixed 64 KiB chunks and passes each checked chunk directly to the caller's
consumer. It does not retain the response body itself. A consumer rejection,
excessive body, unexpected status, invalid request target, or Windows failure
returns a closed error and every native handle closes through RAII.

## Boundary rules

The shared transport does not validate an application's URL, select an origin,
parse text, write a file, check a hash, verify a catalogue, install a release,
or start a process. Those belong respectively to the portable network values,
calling host policy, text service, update-download adapter, catalogue-signature
adapter, installer, and explicit launcher.

See [HTTPS text fetch](NETWORK.md), [update catalogue](UPDATE_CATALOGUE.md),
and Decision 0166.
