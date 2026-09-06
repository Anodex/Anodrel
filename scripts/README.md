# Scripts

This directory contains repeatable development, verification, packaging, and
release helpers.

Scripts must be safe to run from a clean checkout, document their prerequisites,
and avoid writing secrets or generated output into tracked source directories.

## start-linux-session-window-lab.sh

Builds the fixed first-party held Linux child and opens the development-only
Linux child/view Session Lab on a little-endian Wayland desktop. It accepts no
application content or child argument; see `docs/LINUX_WINDOW_SESSIONS.md`.

## check-source-size.ps1

Checks every tracked maintained source and documentation file against the
repository's 550-line organization limit. It reads files only and exits with a
failure that names each file exceeding the limit.

~~~powershell
.\scripts\check-source-size.ps1
~~~

## check-native-ownership.ps1

Checks the native workspace against Decision 0005. It uses locked Cargo metadata
and `native/Cargo.lock` to require only first-party `anodrel-*` packages, local
dependency paths beneath `native/`, and no external package sources. It reads
metadata and tracked files only; it installs nothing and makes no network,
trust, installation, or desktop-UI change.

~~~powershell
.\scripts\check-native-ownership.ps1
~~~

## check-typescript-ownership.ps1

Checks the TypeScript workspace against Decision 0005. Every application and
package runtime dependency must be a local `@anodrel/*` workspace package at
the same version. It also locks the root to the reviewed TypeScript compiler,
Node type definitions, and their one type-only transitive package. It reads the
committed manifests and lockfile only; it installs nothing and makes no network,
trust, installation, or desktop-UI change.

~~~powershell
.\scripts\check-typescript-ownership.ps1
~~~

## verify-windows-release.ps1

Runs the complete non-interactive Windows release evidence set: formatting,
TypeScript and native ownership, strict native lint, source-size, documentation links,
whitespace, the native workspace tests, the release-only frame budget, and the
sample host startup report. It makes no trust, installation, network, or
desktop-UI change. It cannot replace the separate manual native consent, UAC,
Start-menu, file-picker, accessibility, and signed-fixture checks in
`docs/WINDOWS_RELEASE.md`.

~~~powershell
.\scripts\verify-windows-release.ps1
~~~

## verify-windows-accessibility.ps1

Builds the exact Windows host and three fixed first-party diagnostic children,
then runs the property, focus, focus-event, Invoke, structure-event, and
live-status-event UI Automation probes in sequence. It verifies the locked
native graph is first-party before building, opens only temporary host-owned
diagnostic windows, and needs an interactive Windows desktop; it creates no
trust, installation, network, package, or persistent user state. It supplements,
but does not replace, manual Narrator and Inspect acceptance.

~~~powershell
.\scripts\verify-windows-accessibility.ps1
~~~

## provision-product-fixture.ps1

Provisions or removes the development-only Windows product fixture described in
`docs/PRODUCT_FIXTURE.md`. It builds the fixture, host, and provisioning helper,
stages a package outside the repository, signs both executables with a locally
generated development certificate, installs that certificate into machine trust,
and writes one machine-policy record.

Provisioning and `-Remove` need an elevated PowerShell session, change machine
certificate trust, and are for development machines only. Run it with `-Remove`
when finished.

`-Verify` reports whether the machine record currently validates. It is a query
only, changes nothing, and needs no elevation.

## prepare-installed-product-fixture.ps1

Prepares one signed development installer for the fixed product fixture through
Anodrel's own bundle, manifest, resource-embedding, and signing tools. It needs
an elevated development PowerShell session because it adds a temporary local
certificate to machine trust, but it does **not** install the fixture itself.
The printed signed installer command preserves the native consent and UAC
checks. Follow [the installed fixture guide](../docs/INSTALLED_PRODUCT_FIXTURE.md)
for preparation, acceptance, and removal.
