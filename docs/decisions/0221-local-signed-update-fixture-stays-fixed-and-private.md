# Decision 0221: Local signed update fixture stays fixed and private

**Status:** Accepted

**Date:** 2026-09-12

## Context

Anodrel has a fixed-identity native update-acceptance runner, but a positive
run still depends on an externally prepared HTTPS catalogue and newer installer.
That leaves the catalogue, publication endpoint, and their integration with the
Windows update client unproven. A general local web server, configurable URL,
or arbitrary installer argument would be a dangerous shortcut: it could turn a
development test into an unreviewed update-launch tool.

## Decision

Build one first-party Windows-only local update acceptance fixture for exactly
`org.anodrel.local-update-fixture`. It publishes exactly one attached-CMS
catalogue and exactly one newer signed installer through the Windows HTTP
Server API at `https://localhost:45863`. The fixed paths, initial version,
candidate version, identity, and temporary certificates are compiled or
script constants; no command line, environment value, application protocol,
or rendered surface chooses them.

The fixture uses a publisher certificate for Authenticode and CMS, plus a
separate localhost TLS certificate for the Windows transport binding. Trusting
the TLS certificate does not grant update authority: direct WinHTTP certificate
validation, CMS publisher comparison, installed-record continuity, image
signature verification, embedded-release checks, and the elevated installer
transaction all remain mandatory.

The server accepts exact bodyless `GET` requests for its two fixed paths only.
It never implements configuration, upload, directory traversal, directory
listing, proxying, redirects, arbitrary file serving, a shutdown request, or
application control. It runs only on an explicit operator command and has no
automatic startup, persistence, service, or scheduled task.

## Consequences

- Windows can gain an isolated end-to-end update acceptance path without a
  public endpoint or third-party server runtime.
- Fixture preparation changes local machine certificate and HTTP Server API
  configuration, so it remains elevated and explicitly operator-authorized.
- The ordinary development fixture and no-restart fixture remain untouched.
- Production endpoint operation, certificate custody, timestamping, key
  rotation, automatic scheduling, and product update policy remain separate
  decisions.

## Revisit conditions

Revisit for a production staging service, a different HTTP server boundary,
multiple update channels, certificate rotation, another operating system, or a
user-configurable update source.
