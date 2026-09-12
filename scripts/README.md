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

It supports the inbox Windows PowerShell used by the repository's batch helpers
as well as newer PowerShell releases.

~~~powershell
.\scripts\check-typescript-ownership.ps1
~~~

## verify-windows-release.ps1

Runs the complete non-interactive Windows release evidence set: formatting,
TypeScript and native ownership, strict native lint, source-size, documentation links,
whitespace, the native workspace tests, the release-only frame budget, and the
sample host startup report. `-IncludeIdleReport` adds the fixed 30-second
static-window measurement. `-IncludeAccessibilityReport` adds the six direct
Windows UI Automation probes, which need an interactive desktop and each open
and close a temporary host-owned window. Neither option creates trust,
installation, network, or persistent application state. The verifier cannot
replace the separate manual native consent, UAC, Start-menu, file-picker,
Narrator/Inspect, and signed-fixture checks in `docs/WINDOWS_RELEASE.md`.
It also reruns the TypeScript ownership guard through the inbox Windows
PowerShell so double-clicked batch entry points stay covered.

~~~powershell
.\scripts\verify-windows-release.ps1
.\scripts\verify-windows-release.ps1 -IncludeIdleReport -IncludeAccessibilityReport
~~~

Double-click `start-windows-release-evidence.bat` from the repository root to
run that full release-candidate command without entering its options manually.
It needs an interactive Windows desktop and opens the one 30-second idle window
followed by the six temporary accessibility-probe windows.

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
for preparation, acceptance, and removal. The new signed cleanup helper removes
the package without a restart. Close its result dialog before using `-Remove`;
the internal `installed-fixture-cleanup.ps1` helper invokes signed cache
retirement before certificate removal. Active or invalid caches keep trust
intact. Older installed uninstallers that already scheduled reboot deletion
still need their legacy cleanup; rebuilding does not replace installed images.

Pass `-NoRestartAcceptance` to prepare the separate fixed acceptance fixture.
It uses a distinct app identity and development certificate, so it can prove
helper removal and immediate reinstall without disturbing a legacy fixture.
The full procedure is in [the installed fixture guide](../docs/INSTALLED_PRODUCT_FIXTURE.md).

## verify-no-restart-fixture-removal.ps1

Reads the one fixed no-restart fixture's package, policy record, Installed Apps
key, and Start-menu link after interactive removal. It changes nothing and
proves only that those registered surfaces are absent; it cannot observe the
native consent/UAC interaction, helper dialog, cache retirement, or a reboot.

## prepare-local-update-fixture.ps1

Prepares the distinct `org.anodrel.local-update-fixture` acceptance route. It
builds two fixed signed releases (0.1.0 and 0.1.1), creates a signed local CMS
catalogue, temporarily trusts its publisher and localhost TLS certificates,
and configures a loopback-only Windows HTTPS endpoint. It needs an elevated
PowerShell session and does not install either release or start the local
server. `-Remove` removes only this fixture after its signed package and cache
are absent. See [the local signed update fixture](../docs/LOCAL_UPDATE_FIXTURE.md).

## verify-local-update-fixture.ps1

Read-only verification for the selected fixed 0.1.1 local update fixture. It
checks its prepared signed candidate, selected package, signed child/launcher,
Apps & features key, and Start-menu link. It cannot observe consent, UAC,
server behaviour, or a visible result.
