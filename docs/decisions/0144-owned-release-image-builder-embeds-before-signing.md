# Decision 0144: Build release images by embedding resources before signing

**Status:** Accepted

**Date:** 2026-08-31

## Context

The installer can read and validate fixed release resources from its current
image, but a normal Cargo-produced executable contains no release-specific
resources. Using `rc.exe`, a third-party packager, or a sidecar archive would
weaken Anodrel's ownership goal or separate authority data from the image that
will receive the production signature.

Windows provides `BeginUpdateResourceW`, `UpdateResourceW`, and
`EndUpdateResourceW` for adding raw PE resources to a binary file that is not
executing. Updates are accumulated until `EndUpdateResourceW` explicitly commits
or discards them. Resource changes invalidate any existing Authenticode
signature, so embedding must happen before signing.

## Decision

Build `anodrel-release-image`, a first-party Windows release-authoring tool. It
accepts a build operator's unsigned template executable, strict release manifest
and bundle files, and a new output path. It validates the manifest and bundle
through the same owned installer code, copies the template only to a previously
absent output file, writes the two fixed `RT_RCDATA` resources, then reloads the
output as data-only PE content to compare both exact resource byte sequences.

The tool never overwrites an existing output, updates its own running image,
installs a package, writes machine policy, trusts a certificate, launches an
application, downloads content, or invokes a shell. Its successful output is
**unsigned** and must be signed by the separately selected production identity
before an installer can activate it.

## Consequences

Positive:

- Resource assembly is Anodrel code over direct Kernel32 APIs.
- The exact bytes read by the future installer are checked in the output before
  it reaches the signing step.
- A failed resource update is discarded; no partially committed image is called
  a release.

Tradeoffs:

- Release assembly is Windows-only and requires an executable template that is
  not running.
- The operator must use a new output filename and separately sign the result.
- Authenticode signing itself remains a later owned release-tool boundary.

## Revisit conditions

Revisit if a cross-platform build service is introduced, resource updates prove
too slow for large measured bundles, or a Windows package format replaces PE
resource assembly.
