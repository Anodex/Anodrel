# Decision 0163: Owned release manifests derive facts from checked bundles

**Status:** Accepted

**Date:** 2026-08-31

## Context

The release image builder needs a strict manifest that binds application
identity, executable digest, policy, publisher, and payload. Hand-writing its
identity or digest fields would make a release easy to assemble incorrectly,
and an external manifest generator would weaken the owned release path.

## Decision

Provide `anodrel-release-manifest create <release-plan> <bundle>
<new-manifest>`. The exact version-1 plan supplies only release policy choices:
package version, executable path, publisher fingerprint, capabilities, and
network origins. It cannot supply an application identity or any digest.

The tool parses the owned bundle, obtains the root application manifest and its
declared text content, requires that content's digest and text contract, finds
the planned executable entry, derives all digest and payload facts, renders the
strict final manifest, re-parses it, verifies it against the same bundle, and
writes only a fresh synchronized output file.

## Consequences

- App identity, executable digest, and payload digest cannot drift from the
  bytes later embedded in the signed installer.
- Operator-selected capability, network, version, executable, and publisher
  policy stay explicit, strict, and reviewable in a small plan file.
- The plan is not a trust anchor: embedding and Authenticode signing remain
  mandatory separate boundaries.

## Revisit conditions

Revisit for multiple application manifests, non-text package content,
multi-executable releases, additional manifest versions, or a separately
approved declarative policy format.
