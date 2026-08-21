# Decision 0084: HTTPS text fetch is host-authorized, origin-bound, and direct

**Status:** Accepted

**Date:** 2026-08-21

## Context

Anodrel currently lets a granted application hand one validated HTTPS address to
the operating system's associated handler. That is deliberately a navigation
operation, not data access. Anodex-class applications will also need to obtain
remote data without adding a browser engine, Node.js runtime, embedded
webview, request library, or ambient network authority to every application.

An unrestricted HTTP API would let application content choose arbitrary
destinations, headers, credentials, cookies, redirects, proxy behavior, request
bodies, and unbounded responses. It would be a hidden browser-like runtime and
an easy path to local-network probing or credential disclosure. A broad client
is not necessary to prove a first useful native boundary.

Windows ships WinHTTP as a direct OS HTTP client API. It uses OS Schannel TLS
configuration, does not share browser cookies or cache, and has no interactive
UI. Its handles have explicit lifetime and its request path can disable
redirects, cookies, and automatic authentication. See Microsoft's [WinHTTP
overview](https://learn.microsoft.com/windows/win32/winhttp/about-winhttp),
[security considerations](https://learn.microsoft.com/windows/win32/winhttp/winhttp-security-considerations),
and [timeout documentation](https://learn.microsoft.com/windows/win32/api/winhttp/nf-winhttp-winhttpsettimeouts).

## Decision

Reserve Protocol 1.19 for one `network.fetch_text` operation behind the new
host-issued `network.fetch` grant. It accepts exactly one strict HTTPS URL and
returns only an HTTP status code plus bounded UTF-8 text. The portable seam is:

~~~text
NetworkUrl::parse(url) -> NetworkUrl | NetworkUrlError
NetworkTextService::fetch_text(url) -> NetworkTextResponse | NetworkTextServiceError
~~~

The exact protocol field, URL limits, response limits, stable errors, and
origin policy are defined in `docs/NETWORK.md` before implementation begins.
The application never supplies an allowed-origin list, method, headers, body,
cookie, credential, proxy, redirect rule, certificate choice, request timeout,
or native connection handle. The host selects one to eight HTTPS origins when
it composes the service. A request whose URL lies outside that host policy is
indistinguishable from an unavailable service.

The first Windows adapter will use synchronous direct WinHTTP calls on the
authenticated session worker with fixed ten-second phase timeouts. It will use
no proxy, disable automatic redirects, cookies, and authentication, enable
certificate-revocation checking, retain normal certificate validation, and use
RAII to close each WinHTTP handle. It will not set security flags that ignore a
certificate error. It returns no headers, redirects, certificate data, peer
address, proxy data, timing, native status, or response bytes that fail the
text and size rules.

The first slice does not grant this service to the regular development
templates or product fixture. A development diagnostic may use a host-compiled
fixed origin. Production use requires a separately versioned installed-record
origin policy after the signing and packaging decision; this decision does not
invent one.

## Consequences

- Anodrel gains a direct OS HTTPS foundation without embedding a browser,
  JavaScript runtime, webview, or third-party request library.
- Network authority is explicit, origin-bounded, and host-selected rather than
  ambient application authority.
- The initial response model is intentionally narrow but sufficient for a
  small public JSON or text endpoint.
- A request runs synchronously on its session worker for a bounded duration;
  it cannot create unbounded concurrent work or a callback channel.

## Revisit conditions

Revisit before adding another method, request body, request or response headers,
binary content, streaming, redirect following, cookies, authentication,
proxies, DNS/IP literals, local-network destinations, client certificates,
custom TLS policy, caching, an application-selected origin list, concurrent or
background requests, another operating-system adapter, a development-template
grant, or a production installed-record origin format. Each changes the
security or compatibility boundary.
