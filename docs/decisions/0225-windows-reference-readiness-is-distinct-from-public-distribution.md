# Decision 0225: Windows reference readiness is distinct from public distribution

**Status:** Accepted

**Date:** 2026-09-19

**Amends:** Decision 0139's use of "reference release" as the prerequisite for
new application-host work on Linux and macOS.

## Context

Anodrel is a native-first desktop platform intended to remain open source and
to serve Anodex first. It must prove a genuinely usable Windows host before it
duplicates application-host work on Linux or macOS. Earlier planning treated
public code signing identity, certificate custody, timestamping, hosted
updates, and a commercial release operation as Windows-reference gates.

Those activities matter for a public distributed product, but they do not make
the direct host, protocol, renderer, installer, or local development workflow
more usable. Requiring them before the next host would turn an external
distribution decision into a platform-engineering blocker. It would also blur
the established boundary: local signed fixtures are explicit, removable test
harnesses, never a production identity.

## Decision

Windows remains the reference platform. Before Linux or macOS application-host
work begins, it must meet the evidence-backed Windows reference-readiness gates
in `docs/WINDOWS_RELEASE.md`: documented contracts, repeatable builds and
tests, direct native behaviour, bounded security boundaries, local development
packaging, performance evidence, accessibility evidence, and relevant visible
desktop checks.

Public certificate custody, timestamping, key rotation, a hosted update source,
commercial support operation, and public distribution are optional follow-on
work. They must retain their existing strict security decisions if pursued, but
do not block the open-source platform milestone. The decision does not select
an open-source license; the repository needs an explicit license before it can
state public reuse terms.

## Consequences

- Windows-first remains intact: Linux and macOS application hosts still wait
  for a usable, evidenced Windows reference host.
- Development signing fixtures remain valid local acceptance tools; no public
  certificate authority, external account, or hosted release service is needed
  to advance host engineering.
- Release documentation uses "reference readiness" for platform progress and
  reserves "public distribution" for the separately authorized work.
- A public Windows binary distribution cannot be claimed without explicit
  licensing and its chosen distribution trust operation.

## Revisit conditions

Revisit for a user-scoped development package, a public release channel,
production signing or timestamping, key rotation, a hosted update service, a
new operating-system host, or a change to the Windows-first strategy.
