# Decision 0187: Start-menu links launch a verified product launcher

**Status:** Accepted

**Date:** 2026-09-02

## Context

A selected Anodrel product executable is an authenticated child, not a
stand-alone desktop entry point. The native host creates its one-use bootstrap
record and authenticated pipe before it starts that executable. A Shell Link
that opens the child directly bypasses that host-owned setup and is therefore
not a valid product launch, even when the link itself was written atomically.

The first Start-menu writer used the selected child as its target. No signed
release has been provisioned with that writer, but the contract is wrong and
must not become a release surface.

## Decision

Release-manifest and release-plan version 1.4 add a required `launcher`
descriptor. It names one distinct, contained Anodrel Windows host executable;
the final manifest derives its SHA-256 digest from the checked release bundle.
The launcher and child must both be signed by the release publisher before the
installer promotes the release. Installed-record version 1.23 retains the
launcher path and digest beside the selected child.

Only a selected record carrying that descriptor may create a Start-menu link.
The link targets the selected launcher, uses the selected package root as its
working directory, and carries exactly one generated argument sequence:
`--product-launch <selected-application-id>`. The application identity is
already constrained to the record grammar, so this sequence needs no quoting,
shell parsing, or application-controlled text.

Before it creates a product window, the launcher route compares its own
canonical executable path, locked digest, and Authenticode publisher with the
selected record. It then enters the existing product-session coordinator, which
independently locks and verifies the selected child before delivering its
private bootstrap record. The self-check detects a stale or incorrectly
targeted link; it is not represented as protection from a malicious executable
that already ran. Initial launcher execution relies on the signed installer,
the checked deployment under the machine-owned Program Files hierarchy, and
Windows' normal executable trust controls.

Version 1.22 continues to carry a signed Start-menu filename but no longer
creates a link: it has no valid launcher target. Synchronization removes an
old 1.22 direct-child link after policy selects a record without a launcher.

## Consequences

Positive:

- a Windows Start-menu entry reaches the host that owns private bootstrap,
  authenticated IPC, native UI, and child lifetime;
- the link's executable, working directory, and sole argument all come from
  signed selected policy rather than application input;
- release authoring derives both executable digests from checked bundle bytes;
- a stale direct-child shortcut is retired instead of being presented as a
  working product entry point.

Tradeoffs:

- a product release that wants a Start-menu entry must package and sign an
  Anodrel Windows launcher in addition to its application child;
- launcher compatibility across update and rollback is now a release concern;
- a fully signed fixture is still required to prove the Shell Link through a
  real installed product launch.

## Revisit conditions

Revisit for a machine-wide broker, Windows package identity, a signed launcher
update channel independent from the application package, file activation,
multiple entry points, AUMID, desktop links, or another operating system.
