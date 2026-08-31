# Decision 0143: Require the current installer signature to match its embedded publisher

**Status:** Accepted

**Date:** 2026-08-31

## Context

The current-image resource reader selects fixed release bytes, but loading a PE
resource does not establish who authored that image. An installer must not write
machine policy merely because it found a syntactically valid manifest and bundle
inside a running executable.

Anodrel already owns a narrow Windows Authenticode adapter that returns an opaque
leaf certificate fingerprint only after Windows accepts an executable's embedded
signature. The embedded release manifest already contains the publisher
fingerprint that the installed executable must match.

## Decision

The installer activation gate first obtains its own executable path from the
current process, verifies that image through the existing direct Windows
Authenticode adapter, loads its two fixed current-image resources, and requires
the accepted installer leaf fingerprint to equal the manifest's opaque publisher
fingerprint. It returns the checked embedded release only after all four steps
succeed.

The gate returns no path, certificate subject, fingerprint, native trust status,
resource identifier, or payload detail. A normal unsigned development binary
must fail closed. The release builder must provide a separately signed installer
test image before the positive Windows path can be demonstrated.

## Consequences

Positive:

- The installer, manifest, bundle, and installed executable start with one
  publisher identity rather than four independent claims.
- The existing Authenticode adapter stays the only direct certificate boundary.
- Future install and update code has one explicit signed-release activation
  prerequisite instead of scattered preconditions.

Tradeoffs:

- The developer build cannot exercise a successful activation without an
  operator-provided signing identity and resource-bearing image.
- Publisher-key rotation remains intentionally unsupported until a dedicated
  update-trust decision defines it.

## Revisit conditions

Revisit for publisher-key rotation, an operating-system package identity,
enterprise publisher policy, or a brokered installer model.
