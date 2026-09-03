# Signed product display metadata

**Status:** Version 1.2 display metadata and version 1.3's separately signed
Windows Start-menu name are implemented. Record v1.22 retains the name alone;
record v1.23 additionally retains the verified launcher required to create a
working Start-menu entry.

## Purpose

An application identity and certificate fingerprint authorize an installation,
but neither is suitable human-facing text for Windows. A product must supply
its display name and publisher name explicitly before Anodrel can safely create
a Start-menu entry, an Apps & features registration, or another Windows-owned
product surface.

The names are signed release facts. They are not application protocol input,
runtime configuration, a filesystem name, an executable path, a registry key,
or a certificate subject.

## Version 1.2 display fields

Release-plan version 1.2 and derived `anodrel.release.v1` manifest version 1.2
add the required `product` object:

~~~json
"product": {
  "displayName": "Anodrel Sample",
  "publisherName": "Anodrel"
}
~~~

| Field | Rule |
| --- | --- |
| `displayName` | One UTF-8 value from 1 through 120 bytes. It has no leading or trailing Unicode whitespace, no control or directional-format character, and is displayed exactly as signed. |
| `publisherName` | Uses the same bounds and character rules. It is display text, not a replacement for the signed certificate fingerprint. |

Version 1.2 remains a strict extension of version 1.1: it requires the exact
`updateCatalogue` object alongside `product`. Version 1.0 and 1.1 manifests
and plans keep their existing exact field sets and have no product metadata.
Unknown, missing, duplicate, wrongly typed, or unsafe display fields fail
closed.

## Boundaries

Anodrel does not use either name as a directory name, policy-key component,
shortcut filename, process argument, command, URL, or source of authority.
Future Windows integration derives all machine paths and policy from the signed
application identity and release version, then may show these names only after
the signed manifest gate succeeds.

Changing product metadata in a later signed release does not grant a new
application identity or publisher. The matching installed-record v1.21 carries
it with the selected policy, so a later product surface can change only after
the same identity, signature, and release-policy checks that govern
installation, update, or rollback.

## Version 1.3 Start-menu name

Windows shows a Shell Link's filename, rather than its description, in the
Start menu. Version 1.3 therefore retains version 1.2 and adds a third signed
field:

~~~json
"product": {
  "displayName": "Anodrel Sample",
  "publisherName": "Anodrel",
  "startMenuName": "Anodrel Sample"
}
~~~

`startMenuName` uses the display-text bounds and additionally must be one
Windows-safe filename component: no `/`, `\\`, `:`, `*`, `?`, `\"`, `<`, `>`, or
`|`; no trailing period; no `.` or `..` segment; and no Windows device name
(such as `CON`, `NUL`, `COM1`, or `LPT1`, case-insensitively). It is a signed
filename for a future Start-menu link, not a general display value. Record
v1.22 retains it beside the v1.21 product metadata. Record v1.23 requires the
separate version 1.4 launcher descriptor before the installer may create that
link.

Version 1.21 deliberately has no inferred filename, so it cannot create a
Start-menu entry. Version 1.3 continues to require the exact update catalogue.

## Exclusions

This contract does not create Apps & features records, icons, uninstall
commands, file associations, a custom installer window, translated strings, an
application protocol field, or a code-signing identity. Start-menu registration
is separately defined by `docs/PRODUCT_LAUNCHER.md`.

See [release-manifest authoring](RELEASE_MANIFEST.md), [product registration](PRODUCT_REGISTRATION.md),
[Windows installer](WINDOWS_INSTALLER.md), [product launcher](PRODUCT_LAUNCHER.md),
and Decisions 0181, 0182, 0184, and 0187.
