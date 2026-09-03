# Windows product-registration foundation

**Status:** Installed-record versions 1.21 and 1.22 are implemented, including
the separately signed Start-menu name. The direct Start-menu writer is
implemented; automatic installer composition and Apps & features remain
separate work.

## Purpose

A Windows product surface must follow the same machine-selected release as the
host launcher, including after update or rollback. A version-specific shortcut
target or display name remembered by an installer process would become stale.
Version 1.21 therefore stores signed product display metadata in the selected
machine record beside the validated package and executable facts.

## Record version 1.21

Version 1.21 strictly extends version 1.20. It requires the existing
`updateCatalogue` object and this `product` object:

~~~json
"product": {
  "displayName": "Anodrel Sample",
  "publisherName": "Anodrel"
}
~~~

The two strings use the [signed product display metadata](PRODUCT_METADATA.md)
grammar exactly. The installer renders them only from a signed release-manifest
v1.2. The installed-record parser validates them before it returns a selected
application. Version 1.20 and earlier records remain exact and contain no
product object. Version 1.22 strictly extends 1.21 with its signed Windows-safe
`startMenuName`; only that version may later create the one Start-menu link.

## Boundaries

The host may use this private machine-policy display data only for a later
Windows-owned product surface. It is not serialized into an application
protocol message, returned by a client SDK, used as a capability, or accepted
from an application. Product paths, registry keys, shortcut file names, and
executable selection remain derived from the validated application identity and
record, never these display strings.

The installer and host must re-read the selected record for a product-surface
change. Update and rollback therefore select both the executable target and its
signed display facts atomically through existing machine-policy publication.

## Exclusions

This does not automatically refresh a Start-menu shortcut after a policy
transaction, write an Apps & features entry, copy an uninstaller, add a file
association, define an Application User Model ID, or allow applications to
read or edit registration.

See [installed application records](LAUNCH.md), [signed product display metadata](PRODUCT_METADATA.md),
and Decision 0182.
