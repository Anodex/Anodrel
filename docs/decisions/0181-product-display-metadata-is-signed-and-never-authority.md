# Decision 0181: Product display metadata is signed and never authority

**Status:** Accepted

**Date:** 2026-09-02

## Context

The Windows installer can validate a signed application identity, version, and
publisher fingerprint, but those values are not a professional product name or
publisher label. Deriving display text from identity, certificate subjects, or
filesystem paths would make Windows surfaces misleading, unstable, or unsafe.
Untrusted application configuration must not control installer presentation.

## Decision

Add required `product.displayName` and `product.publisherName` fields to exact
release-plan and release-manifest version 1.2. They are bounded, signed UTF-8
display strings with no surrounding whitespace, control characters, or
directional-format characters. Version 1.2 also retains version 1.1's required
signed update-catalogue source. Earlier formats remain exact and do not gain
implicit metadata.

The values may appear only on future host-owned Windows product surfaces. They
are never used as an application identity, policy key, directory, filename,
process argument, URL, certificate selection, or permission.

## Consequences

- Future Windows registration can present a signed product and publisher name
  without consulting mutable application files or certificate subjects.
- Product-facing metadata changes receive the same review and signature as a
  release's capabilities and network policy.
- Releases that need a Windows product surface must deliberately advance to
  version 1.2 rather than making older manifests ambiguous.

## Revisit conditions

Revisit for localization, a separate publisher-brand policy, signed icons,
multiple product channels, operating-system package identity, another platform,
or a public application metadata capability.
