# Windows product-registration foundation

**Status:** Installed-record versions 1.21 through 1.23 are implemented,
including the separately signed Start-menu name and product launcher. The
direct Start-menu writer and its post-policy installer composition are
implemented. Apps & features has a selected signed-image contract and complete
installer composition; its desktop and signed-fixture acceptance remain
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
`startMenuName`. Version 1.23 requires a separate signed launcher descriptor:

~~~json
"launcher": {
  "path": "bin/anodrel-windows-host.exe",
  "sha256": "64 lowercase hexadecimal characters"
}
~~~

The launcher is a distinct contained host executable. The installer derives
its digest from the checked release bundle, validates it with the selected
package, and requires its Authenticode publisher to match the release before
promotion. Only record 1.23 can create the one Start-menu link; older records
do not target the authenticated child directly. See [product launcher](PRODUCT_LAUNCHER.md).

## Boundaries

The host may use this private machine-policy display data only for a later
Windows-owned product surface. It is not serialized into an application
protocol message, returned by a client SDK, used as a capability, or accepted
from an application. Product paths, registry keys, shortcut file names, and
executable selection remain derived from the validated application identity and
record, never these display strings.

The installer and host must re-read the selected record for a product-surface
change. Update and rollback therefore select the child, launcher, and signed
display facts atomically through existing machine-policy publication.
The Start-menu link is then synchronized as a separate post-policy step; it
cannot replace, roll back, or otherwise change that selected policy.

## Exclusions

Apps & features writes one fixed selected-policy entry and uses the fixed
native removal route defined in [Apps & features](APPS_AND_FEATURES.md). This
does not add a file association, define an Application User Model ID, or allow
applications to read or edit registration.

See [installed application records](LAUNCH.md), [signed product display metadata](PRODUCT_METADATA.md),
[Apps & features](APPS_AND_FEATURES.md), and Decisions 0182, 0187, and 0196.
