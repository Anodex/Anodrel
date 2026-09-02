# Decision 0175: Update catalogues derive release facts from locked images

**Status:** Accepted

**Date:** 2026-09-02

## Context

The signed catalogue tool requires a strict JSON input. Hand-authoring the
application identity, package version, publisher fingerprint, installer length,
and installer digest duplicates facts that are already embedded in and measured
from the signed installer. A typo or stale sidecar would produce a valid CMS
catalogue that simply cannot update the intended release.

## Decision

Add an operator-only catalogue-authoring tool. It accepts one absolute signed
installer image, one explicit HTTPS origin and path selected for release
publication, and one fresh JSON output. Before it renders anything, it locks
the image, validates its embedded Anodrel release and Windows Authenticode
signature, and requires the signer to match the embedded publisher. While that
lock remains alive, it derives the application identity, package version,
publisher fingerprint, byte length, and SHA-256 digest from the image.

The tool renders and re-parses the strict `anodrel.update-catalogue.v1` JSON
before it creates a synchronized fresh output. The separate attached-CMS signer
remains responsible for signing that output. No application, host, protocol,
installer, or updater receives this authoring surface.

## Consequences

- Catalogues cannot be created from a release manifest or digest sidecar that
  disagrees with the signed image intended for distribution.
- The publication location remains an explicit release-operator choice and is
  authenticated only after the resulting catalogue is CMS signed.
- The tool does not host bytes, create certificates, sign a catalogue, write
  machine policy, install, elevate, or launch an image.

## Revisit conditions

Revisit for a repository publisher, multiple channels, detached signing,
timestamping, key rotation, an owned HTTPS release service, another platform,
or a reproducible release pipeline that needs a higher-level orchestration
format.
