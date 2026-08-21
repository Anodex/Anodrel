# Anodrel HTTPS text fetch

**Status:** The portable Protocol 1.19 core, strict URL and exact-origin
values, TypeScript SDK, deterministic mock host, direct WinHTTP adapter, and a
fixed-origin compiled Windows development diagnostic are implemented. An
installed-application origin policy is not implemented.

## Purpose and boundary

Anodrel's first network operation will fetch one bounded UTF-8 text response
from an HTTPS URL a host policy explicitly allows. It is a data request, unlike
`external.open`, which merely asks the operating system to hand an HTTPS link
to a person's associated browser or handler.

The operation is deliberately not a general browser, HTTP client, proxy, or
credential store. It carries no native socket, connection, header, cookie,
certificate, redirect, timing, cache, peer, or callback value across the
application boundary.

## Protocol 1.19

`network.fetch_text` requires the host-issued `network.fetch` grant. Its
payload is exactly:

~~~json
{ "url": "https://api.example.invalid/status?format=text" }
~~~

The successful result is exactly:

~~~json
{ "statusCode": 200, "text": "healthy" }
~~~

`statusCode` is an integer from 100 through 599. It is the HTTP response
status, not a native status and not proof that the application accepted the
text. The host returns a success envelope even for a non-2xx HTTP response when
its bounded text body is valid; applications decide how that HTTP result fits
their own protocol. No response headers are returned.

The host returns `network.unavailable` when no text service is attached, its
origin policy does not allow the URL, the request cannot complete within its
fixed bounds, or the operating system cannot complete it. It returns
`network.response_invalid` only when a received response cannot be represented
by this protocol's bounded UTF-8 result. Neither error carries the URL, native
status, header, certificate, address, proxy, or response bytes.

## URL and origin rules

A URL is one ASCII value of at most 2,048 bytes. It must:

- begin exactly with `https://`;
- contain one DNS-style hostname and an optional port from 1 through 65,535;
- contain no user information, IP literal, backslash, whitespace, control
  character, malformed percent escape, or fragment; and
- retain its path and query as opaque request-target text.

The host constructs its origin policy before authentication. An origin contains
only an HTTPS DNS hostname and its effective port (443 when omitted). It never
contains a path, query, fragment, header, wildcard, IP range, or application
supplied value. The first service accepts from one through eight exact origins.
The URL's canonical hostname and effective port must match one configured origin
before any Windows network API call. Origin rejection maps only to
`network.unavailable`.

## Request and response bounds

The host issues exactly one `GET` request. It uses one host-fixed user-agent and
accepts no request header, method, body, form, file, cookie, referrer,
credential, client certificate, proxy setting, redirect policy, or timeout from
the application.

Each resolving, connecting, sending, and receiving phase has a fixed ten-second
timeout. The first adapter follows no redirect, has no proxy or automatic proxy
discovery, does not send or retain cookies, and does not perform automatic
authentication. It uses ordinary Windows certificate validation plus
certificate-revocation checking. It never weakens TLS verification to make a
connection succeed.

The complete response text is at most 32 KiB encoded as UTF-8. A response that
exceeds that limit, is not valid UTF-8, or has a status outside the documented
range cannot produce a partial result. The host does not inspect, transform,
cache, persist, render, log, or otherwise expose headers or text beyond the
one successful protocol result.

## Windows mapping

The direct adapter uses WinHTTP: `WinHttpOpen`, `WinHttpConnect`,
`WinHttpOpenRequest`, `WinHttpSendRequest`, `WinHttpReceiveResponse`, status
query, bounded reads, and `WinHttpCloseHandle`. It uses no browser, webview,
Node.js, WinINet, COM browser component, or third-party network library.

Every session, connection, and request handle has one RAII owner. Parent and
child handles are closed on every success, rejection, timeout, and failure path.
Each request creates a fresh direct no-proxy session, sets every phase timeout
to ten seconds, then disables cookies, redirects, automatic authentication, and
keep-alive on its request handle before sending. It enables Windows
certificate-revocation checking on that same handle and sets no
certificate-error-ignore flag. The adapter maps all native failure detail to
the two stable safe errors above. It does not start a callback, background
worker, or UI operation; the existing authenticated session worker holds the
bounded synchronous work.

## Fixed-origin Windows development diagnostic

The explicit `--native-network-sample-client <native-client.exe>` Windows-host
route exists only to exercise the complete direct transport boundary. The
operator names an unverified compiled diagnostic executable; the host supplies
its one private bootstrap invitation and grants exactly `network.fetch`.
Before authentication, the host constructs a service with the one compiled
origin `example.com:443`. The first-party diagnostic child itself requests only
the compiled URL `https://example.com/`, validates that the protocol response
contains exactly a representable status and bounded text, then exits without
printing or retaining the response.

This route does not accept a URL, origin, method, header, body, proxy, timeout,
or certificate option from its command line or from the diagnostic child. A
different executable selected by an operator could still request another valid
path at that one fixed origin, so this is a development diagnostic rather than
a product launch or template grant. Regular native templates, Node samples,
the signed development fixture, and installed application sessions do not
receive this service. The external request needs ordinary outbound Internet
access; a failure is a safe diagnostic failure rather than evidence that an
application session has network authority.

## Deferred work

POST, headers, binary data, streams, uploads, multipart data, redirects,
cookies, authentication, client certificates, proxies, cache, HTTP/3 policy,
web sockets, local and private-network access, response metadata, request
cancellation after execution begins, concurrent requests, reusable connections,
application-selected origin policy, production installed-record policy, and
non-Windows adapters are all deferred.

See Decision 0084 and `docs/THREAT_MODEL.md`.
