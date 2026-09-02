# Windows release-manifest authoring

**Status:** Implemented first-party manifest-authoring boundary.

## Purpose

`anodrel-release-manifest` creates the strict `anodrel.release.v1` manifest
that an owned release image later embeds and signs. It uses only Anodrel code;
it is not a package manager, archive builder, signer, installer, or updater.

The operator supplies an absolute release-plan file, an absolute checked bundle,
and a previously absent absolute manifest output. The tool derives the
application identity, application content check, executable digest, bundle
length, and bundle digest from the bundle. A plan cannot supply or override
those facts.

## Release plan format

`anodrel.release-plan.v1` is strict UTF-8 JSON. Version 1.0 has exactly these
fields:

~~~json
{
  "formatVersion": { "major": 1, "minor": 0 },
  "packageVersion": { "major": 1, "minor": 0, "patch": 0 },
  "executable": { "path": "bin/product.exe" },
  "publisher": {
    "leafCertificateSha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
  },
  "capabilities": ["ui.document.write", "ui.events.read", "session.close"],
  "networkOrigins": []
}
~~~

Version 1.1 adds exactly one `updateCatalogue` object:

~~~json
"updateCatalogue": {
  "origin": { "host": "updates.example.test", "port": 443 },
  "path": "/anodrel/catalogues/stable.p7s"
}
~~~

The format version is exactly 1.0, 1.1, or 1.2. Version 1.1 requires this
source and derives a final release manifest at 1.1; version 1.0 carries no
update source. Version 1.2 retains this source and adds product display
metadata. Package-version fields are unsigned 16-bit integers. The executable
path, capability names, network origins, and catalogue location undergo the
same strict validation as the final release manifest. The publisher is one
lowercase SHA-256 leaf-certificate fingerprint. Unknown, duplicate, missing,
or extra fields fail closed.

Version 1.2 adds the required signed `product` object while retaining version
1.1's required catalogue source:

~~~json
"product": {
  "displayName": "Anodrel Sample",
  "publisherName": "Anodrel"
}
~~~

Its two bounded display values are for later host-owned Windows product
surfaces only; they are never a path, application identity, policy key, or
certificate selector. See [signed product display metadata](PRODUCT_METADATA.md).

## Creation contract

~~~text
release plan + checked anodrel.bundle.v1
                    |
                    v
read root anodrel.application.json and its declared text content
derive application ID, executable digest, and payload facts
                    |
                    v
render and re-parse one strict anodrel.release.v1 manifest
                    |
                    v
write and synchronize one new manifest file
~~~

The bundle must contain exactly the root `anodrel.application.json`, its
declared valid UTF-8 text content with the declared digest, and the exact
release-plan executable path. The final manifest is parsed and checked against
the same bundle before its output file is created.

The command is:

~~~text
anodrel-release-manifest create <release-plan.json> <bundle.bin> <new-manifest.json>
~~~

It never overwrites or alters an input, extracts a bundle, chooses a
certificate, signs, creates trust, embeds a resource, installs, launches,
downloads, or starts an update. On output write or synchronization failure it
removes only the newly created output file.

## Compatibility

The plan is an authoring input, not runtime policy and not a signed artifact by
itself. Its values become authoritative only after the derived final manifest
is embedded in an image and that image is signed by the same publisher
fingerprint. See [update discovery](UPDATE_DISCOVERY.md), [release bundle](RELEASE_BUNDLE.md),
[release image](RELEASE_IMAGE.md), [signing](SIGNING.md), and Decision 0163.
