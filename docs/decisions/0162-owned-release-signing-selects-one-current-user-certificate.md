# Decision 0162: Owned release signing selects one current-user certificate

**Status:** Accepted

**Date:** 2026-08-31

## Context

Anodrel can author its release bundle and resource-bearing installer image, but
the installer must have an Authenticode signature before it can activate any
machine transaction. Delegating that final step to a signing shell, installer
framework, or remote service would weaken the project's direct ownership
boundary.

Windows exposes Authenticode signing through `SignerSignEx` in `Mssign32.dll`.
It requires application-defined structures and dynamic lookup. The product
owner has not selected a certificate authority, private-key custody provider,
or timestamp service, and Anodrel must not select or trust one implicitly.

## Decision

Provide one first-party `anodrel-release-sign` command that copies one verified
Anodrel release image to a fresh absolute output path and signs only that copy.
The operator supplies exactly one lowercase SHA-256 certificate fingerprint.
The tool selects that exact certificate from the current user's Windows `MY`
store, with no subject lookup, certificate picker, store mutation, or fallback.
Before it creates an output, the checked embedded release manifest must name
the same opaque publisher fingerprint.

The tool calls `SignerSignEx` directly with SHA-256 and no timestamp URL. It
then requires Windows Authenticode to accept the output and requires the
accepted leaf fingerprint to equal the originally selected certificate. A
failed signing or verification removes only the fresh copy the current call
created. The copy is bounded to 576 MiB: the 512 MiB release-payload limit plus
a 64 MiB PE envelope. It never signs an input in place, creates a certificate,
imports a key, installs trust, starts an installer, or contacts a network
service.

## Consequences

- The final signed image is produced by Anodrel and Windows APIs only.
- Certificate selection is reproducible and non-interactive, but the operator
  must provision an accessible current-user signing certificate separately.
- Version 1 signatures have no timestamp. A production release needs an
  approved timestamp policy before certificate expiry can be handled honestly.
- A development self-signed certificate remains a manual, removable fixture;
  this tool never changes its trust relationship.

## Revisit conditions

Revisit for an approved timestamp authority, hardware-backed or remote key
provider requirements, current-machine or service certificate custody,
publisher rotation, nested signatures, or a non-PE distribution format.
