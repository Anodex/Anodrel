# Decision 0099: Installed network origins remain machine-selected

**Status:** Accepted

**Date:** 2026-08-21

## Context

Decision 0084 established Anodrel's direct, bounded HTTPS text-fetch service.
The service already requires both the `network.fetch` capability and an exact
host-created HTTPS origin policy, but it is intentionally attached only to one
compiled development diagnostic. A signed installed application needs a way to
receive that same narrow service without letting its package, executable,
protocol messages, or rendered UI select network authority.

The installed application record is the existing machine-selected source of
application identity and capabilities. Its strict versioning means a new
network policy field can be introduced without changing what an older record
means.

## Decision

Add installed record version **1.14**. It requires a top-level
`networkOrigins` array alongside the existing `capabilities` array. Every entry
is exactly:

~~~json
{ "host": "api.example.test", "port": 443 }
~~~

`host` uses the already-defined DNS-host grammar and is canonicalized by the
portable `NetworkOrigin` value. `port` is a whole number from 1 through 65,535.
The array is ordered, contains no duplicate canonical host-and-port pair, and
has at most eight entries.

`network.fetch` is a version 1.14 capability. A record granting it must name
one through eight `networkOrigins`; a record that does not grant it must name
an empty array. This keeps an allowed origin coupled to an actual capability
and prevents an unused allowlist from becoming latent authority.

The portable installed-record validator converts that array into the existing
`NetworkOriginPolicy`. The registered Windows-session composition attaches the
existing direct WinHTTP service only when that validated policy is present.
The protocol, SDK method, URL grammar, response limit, request method, TLS
policy, error categories, and no-proxy behavior from Decision 0084 stay
unchanged.

Neither the record's raw origins nor the resulting policy are a protocol result
or a renderer value. An application cannot enumerate the policy, select an
origin, add a path rule, alter it at run time, choose headers or a request body,
or observe why a URL was rejected. Only a trusted Windows installer or
administrator may write the existing machine registry record.

## Consequences

- A verified installed application can use the existing bounded HTTPS text
  service only when machine policy deliberately grants it and names exact
  origins.
- Older records cannot gain network authority by being read by a newer host.
- Network policy validation stays portable and testable, while Windows service
  construction remains inside the Windows registered-session adapter.
- The path still does not validate a production signing identity, install a
  package, or make the development fixture a production release.

## Revisit conditions

Revisit before adding wildcard hosts, IP literals, paths, query rules,
per-origin methods, headers, bodies, credentials, proxies, redirects, local
network exceptions, application-managed configuration, policy readback,
runtime changes, another network service, or a non-Windows adapter. Each would
change the authority and compatibility boundary.
