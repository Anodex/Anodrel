# Decision 0183: Start-menu registration begins with selected-policy proof

**Status:** Accepted

**Date:** 2026-09-02

**Superseded in part:** Decision 0187 replaces the later writer's direct-child
target with a separately verified Anodrel product launcher.

## Context

A Windows Start-menu link is a product-facing launch surface. A path held by an
installer process, a mutable package manifest, or an application request could
be stale or point at an untrusted executable after update or rollback. The
selected machine record is the only current launch authority and now has signed
display metadata at version 1.21.

## Decision

Require an opaque preflight before any direct Shell Link operation. It verifies
the current signed installer, selects one fixed machine record by the signed
application identity, validates the selected executable's Authenticode signer
against its record and the installer, and requires signed record product
metadata. Its later writer may derive only the all-users Common Programs link
under `Anodrel\\<application-id>.lnk`.

No caller selects a target, label, publisher, icon, argument, working
directory, filename, registry data, Application User Model ID, or COM object.
Legacy records without v1.21 metadata do not get an inferred link.

## Consequences

- Install, update, and rollback can eventually refresh one link from current
  policy instead of retaining installer-time paths.
- The first Shell Link implementation has an independently testable security
  boundary before hand-written COM is introduced.
- Product metadata stays display-only despite becoming eligible for a Windows
  surface.

## Revisit conditions

Revisit for icons, localized labels, Apps & features, a desktop link, taskbar
integration, packaged identity, a product launcher, another scope, or another
platform.
