# Windows Apps & features registration

**Status:** The signed product metadata, selected-policy Start-menu route, and
bounded uninstall transaction are implemented. The Apps & features
selected-policy preflight and fixed registry writer are implemented but are not
yet composed into installation or removal while the fixed native consent and
UAC route remains separate work.

## Purpose

Apps & features must remove the same release that Windows machine policy
selects. It cannot point at a downloaded installer, an application child,
mutable package content, or a user-provided command. The release version is
also important: a signed installer for an old version must not uninstall a
newer selected release with the same publisher.

## Fixed installed image

Before an installer promotes a checked private stage, it will place one
byte-verified copy of its current signed image at this fixed location:

~~~text
<selected package root>\uninstaller\anodrel-windows-installer.exe
~~~

The directory name and filename are Anodrel constants. They are not release
manifest fields, bundle entries, display metadata, application protocol data,
or command-line input. The copy comes only from the current installer after
its locked signature and embedded-release checks succeed, then Windows must
also accept the copied file's signature. This avoids a separate uninstaller
format while keeping the copy outside the self-referential release bundle.

## Selected-policy registration

After machine policy selects a complete release, the installer will re-read
that record and verify the fixed image inside its selected package root. The
verification requires all of the following:

1. The fixed image and each relevant directory are ordinary, non-reparse
   objects below the canonical selected package root.
2. Windows accepts the image's Authenticode signature and its publisher equals
   both selected policy and the image's embedded signed release.
3. The image's embedded application identity equals selected policy.
4. The image's embedded package version equals the selected canonical version
   directory.
5. Selected policy retains signed product display metadata.

Only this proof can create or update the one machine key:

~~~text
HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\Anodrel\<application-id>
~~~

The identity remains the key component. The registry values `DisplayName`,
`Publisher`, and `DisplayVersion` are display-only signed facts. The writer
will set one quoted path to the verified installed image plus its fixed owned
uninstall route. It will also declare no repair or modification route. It will
not write an icon, file association, AUMID, usage count, telemetry value,
application-controlled command, or arbitrary registry data.

## Lifecycle

Install, update, and rollback register only after their policy publication
succeeds. If registration fails, the selected record remains authoritative and
the operation reports incomplete product registration; it does not roll policy
back. Uninstall removes the verified Apps & features key and Start-menu link
before it removes policy and then every other package file. It retains only its
executing fixed image and ancestor directory, then uses direct Windows delayed
removal to delete that image, the empty directory, and the empty package root
at the next restart. A missing registration key is harmless. An unsafe or
undeletable key leaves selected policy untouched.

Apps & features invokes the installed image's fixed interactive removal route.
It obtains new native consent and a fixed UAC handoff before it executes the
already-elevated removal command. A zero child exit still will not be presented
as proof until the selected policy is absent. See [uninstall flow](UNINSTALL_FLOW.md).

## Exclusions

This contract does not add repair, arbitrary uninstall arguments, silent
uninstall, background removal, a separate updater service, a package manager,
an installation date, custom icons, a desktop shortcut, telemetry, or
application access to registration.

See [product registration](PRODUCT_REGISTRATION.md), [Windows installer](WINDOWS_INSTALLER.md),
[Windows release readiness](WINDOWS_RELEASE.md), [uninstall flow](UNINSTALL_FLOW.md),
and Decisions 0196 through 0198.
