# Signed product display metadata

**Status:** Version 1.2 parsing and first-party manifest authoring are
implemented. Windows registration surfaces remain separate work.

## Purpose

An application identity and certificate fingerprint authorize an installation,
but neither is suitable human-facing text for Windows. A product must supply
its display name and publisher name explicitly before Anodrel can safely create
a Start-menu entry, an Apps & features registration, or another Windows-owned
product surface.

The names are signed release facts. They are not application protocol input,
runtime configuration, a filesystem name, an executable path, a registry key,
or a certificate subject.

## Version 1.2 fields

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
application identity or publisher. It only updates a product surface after the
same identity, signature, and release-policy checks that govern installation or
update.

## Exclusions

This contract does not create shortcuts, Apps & features records, icons,
uninstall commands, file associations, a custom installer window, translated
strings, an application protocol field, or a code-signing identity.

See [release-manifest authoring](RELEASE_MANIFEST.md), [Windows installer](WINDOWS_INSTALLER.md),
and Decision 0181.
