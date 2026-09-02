# Decision 0168: Signed installed policy selects update catalogue source

**Status:** Accepted

**Date:** 2026-09-01

## Context

The owned updater has a signed catalogue format and a checked image downloader,
but neither can safely choose an initial catalogue endpoint. Reusing the
application `network.fetch` grant would let application policy affect product
delivery and would fail to provide an update route for applications that do not
have network authority. Reading an arbitrary configuration value would widen
the updater into a general network client.

## Decision

Add an exact `updateCatalogue` location to release-manifest version 1.1 and
machine-record version 1.20. A release manifest version 1.0 and record version
1.19 remain valid and intentionally carry no update source. Version 1.1/1.20
requires one exact HTTPS origin and strict `.p7s` path; the installer renders
it only after the usual signed release and private staging gates pass.

The native updater accepts a host-selected installed application identity,
reloads that one fixed record, verifies its executable with Windows
Authenticode, requires the signer to match policy, and retrieves at most one
128 KiB attached-CMS catalogue from the recorded location through the shared
direct transport. It verifies the catalogue against that installed signer and
then sends it through the existing newer-candidate preflight.

## Consequences

- Product update delivery does not depend on an application network grant or
  accept an arbitrary endpoint.
- Older releases stay installable and operational but cannot opt into automatic
  catalogue discovery retroactively.
- A signed release changes its own future update endpoint only through the
  existing forward-version installer transaction.
- Cache selection, consent, installer process handoff, recovery, scheduling,
  endpoint migration, and key rotation remain separate decisions.

## Revisit conditions

Revisit for multiple channels, fallback origins, a redirect policy, endpoint
migration, certificate rotation, an emergency recovery route, another platform,
or a public update API.
