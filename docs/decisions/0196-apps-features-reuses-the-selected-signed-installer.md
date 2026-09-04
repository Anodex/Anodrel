# Decision 0196: Apps & features reuses the selected signed installer

**Status:** Accepted

**Date:** 2026-09-04

## Context

Anodrel has a bounded machine uninstall transaction, but the installer image
that proves its embedded identity and publisher can otherwise remain in a
download location. Windows' Apps & features surface needs a stable command
that survives restart, update, and rollback. A registry value must never point
at a download, mutable package metadata, an application-selected executable,
or a general command line.

Packaging a separate uninstaller would add a second release image and signing
surface. Pointing the registration at an arbitrary previous installer would
also allow an older release to remove a newer selected package.

## Decision

Each promoted machine release will contain exactly one copy of the current,
already verified signed Anodrel installer image at the installer-derived path:

~~~text
<selected package root>\uninstaller\anodrel-windows-installer.exe
~~~

The copy is written into the private staging tree before promotion, with no
overwrite or caller-selected source, name, or destination. Its bytes must
match the locked current image and Windows must accept its Authenticode
signature before that stage can be promoted. It is intentionally outside the
release bundle: an installer cannot include a copy of its own final signed
image without a self-referential payload. The copied image remains protected
by the already verified source image, byte-for-byte verification, direct
signature verification, and the normal machine-owned Program Files tree.

Apps & features registration will be a separate, post-policy operation. It
will read fresh selected policy and verify the fixed installed image before it
writes one all-users key below:

~~~text
HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\Anodrel\<application-id>
~~~

The writer may use only signed product display metadata, the release version,
and the fixed installed image. Its uninstall command will contain one quoted
fixed image path and a fixed Anodrel-owned uninstall route; it accepts no
application, registry, product, network, or caller-controlled argument. The
same selected-policy proof will remove the key before package cleanup. Update
and rollback will refresh it only after their machine-policy transition.

The fixed installed image must prove that it belongs to the selected package
version as well as the selected application identity and publisher. An older
signed installer image may therefore not remove a newer release merely because
both share a publisher.

## Consequences

- Apps & features gains a durable first-party uninstall route with no installer
  framework or package manager.
- An update or rollback switches both the selected package and its product
  registration only after policy points at a complete release.
- The release installer image becomes part of each promoted version directory;
  the bounded payload limit and staging-space estimates must account for it.
- A later custom uninstaller, repair route, per-user installation, package
  identity, icon, installation date, or localization needs a new decision.

## Revisit conditions

Revisit for a separately signed uninstaller, a stable machine broker,
packaged Windows distribution, per-user installation, repair, product icons,
multiple channels, localized product surfaces, or another operating system.
