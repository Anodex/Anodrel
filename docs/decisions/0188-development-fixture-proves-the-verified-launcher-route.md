# Decision 0188: Development fixture proves the verified launcher route

**Status:** Accepted

**Date:** 2026-09-02

## Context

Decision 0187 corrected the Windows Start-menu contract: an Anodrel child must
be reached through a separately verified host launcher, not started directly.
The existing development fixture predates that decision. It stages and signs
only its child and writes record v1.2, so it can test `--product-session` but
cannot test the launcher's self-verification and handoff into that session.

That leaves the new production launch path unexercised until a fully packaged
release exists. The existing fixture is the deliberately restricted place to
close the host-route gap without introducing a general development launch API.

## Decision

The fixed development fixture will stage a separately copied
`anodrel-windows-host.exe` alongside its child. The provisioning script signs
both files with the same temporary development certificate. The provisioning
helper measures both digests, verifies both Authenticode signatures, requires
the same leaf publisher fingerprint, and writes record v1.23 with the fixed
launcher descriptor.

Record v1.23 also requires product and update-catalogue metadata. The fixture
uses compile-time display values and a reserved development catalogue location
only to satisfy that strict record shape; it does not create a Start-menu link,
installer image, update, or network request. Its policy record remains a
development-machine input, not a signed production release manifest.

The only new activation check is an explicit operator command that starts the
staged launcher with the fixed `--product-launch` fixture identity. It proves
the launcher's canonical-path, locked-digest, and publisher checks before the
existing child-session coordinator runs. It accepts no custom identity, child
path, record path, or argument.

## Consequences

Positive:

- the development fixture can exercise the same launcher-to-child lifetime a
  future Start-menu entry uses;
- the helper refuses a missing, oversized, unsigned, or differently signed
  launcher before it can write machine policy;
- the fixture continues to use Windows APIs and Anodrel code only.

Tradeoffs:

- the fixture package contains two signed executables instead of one;
- its record has newer metadata that is not evidence of a signed production
  manifest or an installed application;
- the final Explorer and installer path still needs a signed installed fixture
  because this route intentionally creates no Windows shortcut.

## Revisit conditions

Revisit for a real installed development fixture, a release-image test
harness, package identity, a broker, file activation, multiple entry points,
or an operating-system-specific product launcher.
