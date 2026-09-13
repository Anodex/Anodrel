# Decision 0223: Start-menu icons are host-owned brand assets

**Status:** Accepted

**Date:** 2026-09-12

**Supersedes in part:** Decisions 0183 and 0186's exclusion of a custom
Start-menu icon.

## Context

Anodrel windows already render the platform mark from first-party brand
geometry, but a product shortcut without an explicit icon is displayed by
Windows as a generic executable. A product-facing shell entry should carry the
same clear platform identity as the product window.

An application-controlled icon path, image payload, resource index, or shell
property would make branding a mutable capability surface and could mislead a
person about what will launch.

## Decision

The Windows installer renders one fixed multi-size `.ico` file from the
first-party Anodrel brand geometry. It writes that file atomically as
`Anodrel.ico` only under the verified all-users `Common Programs\Anodrel`
directory, then sets every Anodrel product link's icon location to that exact
file and resource index zero.

The link writer derives both the directory and icon name as constants. It
accepts no product icon, package icon, resource index, image decoder, path,
application protocol field, or command-line value. The shared icon remains
after a single product uninstall because other Anodrel entries may still use
it. It contains no application data and grants no authority.

## Consequences

- Start-menu entries visibly use the Anodrel mark at Windows-supported sizes.
- The Start-menu and native window icons share one first-party visual source.
- Product packages do not need a mutable image file or a new release-record
  field merely to select a shell icon.

## Revisit conditions

Revisit for a product-specific signed identity mark, localized shell metadata,
Windows packaged identity, per-user installation, or another operating system.
