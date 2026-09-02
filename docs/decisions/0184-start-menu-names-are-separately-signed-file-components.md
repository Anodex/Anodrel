# Decision 0184: Start-menu names are separately signed file components

**Status:** Accepted

**Date:** 2026-09-02

## Context

Windows displays a Shell Link's file name below its Start-menu icon. The
version 1.2 `product.displayName` is intentionally general display-only text:
it may not become a filename. Naming the link from an application ID instead
would make a safe but unprofessional Windows surface.

## Decision

Add required `product.startMenuName` to strict release-plan and
release-manifest version 1.3. It is separately signed, bounded text that also
meets the Windows file-component grammar: no separators, reserved characters,
trailing period, dot segment, or device name. Version 1.3 retains the version
1.2 display names and update catalogue.

Record version 1.22 strictly extends 1.21 with the same third product field.
Only a selected 1.22 record will become eligible for a later all-users
Start-menu link. The name forms the one visible `.lnk` filename under the
fixed `Common Programs\\Anodrel` directory. It is never a policy identity,
directory, argument, executable path, registry key, URL, certificate selector,
or application protocol field.

## Consequences

- Windows can show a professional signed product name without treating general
  display text as a filesystem path.
- Version 1.21 remains valid product metadata but deliberately does not create
  a Start-menu link.
- Release authors must make the Windows-visible name explicit before using
  product registration.

## Revisit conditions

Revisit for localization, multiple shortcuts, product channels, a signed icon,
desktop links, packaged identity, Apps & features, or another platform.
