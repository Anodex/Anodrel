# Decision 0218: Fixture trust remains through delayed uninstaller cleanup

**Status:** Trust-lifetime invariant retained; restart-only mechanism superseded
by Decision 0219. New fixture cleanup retires verified helper caches before
removing trust. Legacy scheduled deletions retain the procedure below.

**Date:** 2026-09-08

The historical restart-only mechanism below applies only to legacy installed
images. Current fixture removal follows Decision 0219's signed helper/cache
path and Decision 0220's isolated acceptance procedure.

## Context

The fixed installed uninstaller removes its own final signed image only at the
next Windows restart. The development-fixture preparation script previously
allowed its temporary certificate trust to be removed as soon as selected
machine policy disappeared. That could leave the scheduled cleanup image on
disk without the trust required for its own verification or later diagnostic
use.

## Decision

`prepare-installed-product-fixture.ps1 -Remove` must require both an absent
fixture policy record and an absent fixed installed package directory before it
removes local fixture output or development certificate entries. A remaining
directory is a closed failure that tells the operator to restart Windows; the
script does not remove, rename, or inspect that directory.

For this fixed fixture, the package directory is exactly:

~~~text
<Program Files>\Anodrel\Applications\org.anodrel.product-fixture\0.1.0
~~~

The application-identity parent is not the selected package directory. It may
remain empty after Windows has completed the scheduled removal and must not
prevent cleanup of the development certificate.

The normal preparation route applies the same absence check before assembling a
new fixture. It therefore refuses a same-version reinstall over incomplete
restart-delayed cleanup.

## Consequences

- Temporary trust survives until Windows has removed the final signed image.
- The fixture script cannot delete a Program Files package or bypass the
  installed uninstaller's signed cleanup lifecycle.
- Fixture removal now has an explicit restart step before trust removal.

## Revisit conditions

Revisit for a separate signed cleanup helper, a different package layout,
per-user installation, a production certificate lifecycle, or another platform.
