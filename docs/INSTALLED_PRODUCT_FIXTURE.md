# Installed development product fixture

**Status:** Development-only Windows acceptance preparation. This is not a
public installer, release channel, SDK feature, or production distribution.

## Purpose

The staged product fixture proves the verified child-to-host session path. This
separate procedure prepares one signed native installer that carries that same
fixed fixture through Anodrel's complete Windows release chain:

~~~text
fixed package
    -> owned bundle
    -> derived release manifest
    -> resource-bearing installer image
    -> Authenticode-signed installer
    -> explicit native consent and UAC
    -> Program Files package + machine record + Start-menu launcher
~~~

It exists to make the final Windows installer, Explorer, product-launcher, and
uninstall acceptance checks repeatable before Anodrel has a production signing
identity. It does not replace the production identity, timestamp, update, or
release-operation decisions in [Windows release readiness](WINDOWS_RELEASE.md).

## Fixed development scope

The preparation script has no product, package, capability, certificate,
output, or installer-command parameters. It always uses these values:

| Item | Value |
| --- | --- |
| Application ID | `org.anodrel.product-fixture` |
| Package version | `0.1.0` |
| Child | `bin/anodrel-product-fixture.exe` |
| Launcher | `bin/anodrel-windows-host.exe` |
| Start-menu name | `Anodrel Product Fixture` |
| Grants | `ui.document.write`, `ui.events.read`, `session.close` |
| Update catalogue | Reserved development location; no request is made |
| Local output | `%LOCALAPPDATA%\Anodrel\InstalledProductFixture` |

The script builds only first-party Anodrel binaries. The bundle is Anodrel's
bounded raw format, resource embedding uses direct Windows resource APIs, and
signature verification, installation, registration, and removal use the direct
Windows adapters already used by the platform. No installer framework, archive
format, webview, Node runtime, or third-party desktop runtime is involved.

## Prepare the signed installer

This is an explicit development-machine trust change. The script creates or
reuses one local certificate in the current user's personal certificate store
and adds that exact certificate to `LocalMachine\Root` and
`LocalMachine\TrustedPublisher`. It needs an elevated PowerShell session and
reverses those entries during removal.

Before using it, make sure the staged product fixture is not selected. If you
previously ran `provision-product-fixture.ps1`, remove that fixture first; the
two procedures share an identity and an initial installation must refuse an
existing selected policy.

From an **elevated** PowerShell session at the repository root:

~~~powershell
.\scripts\prepare-installed-product-fixture.ps1
~~~

The script builds and stages the fixed package, signs the child and launcher,
authors the bundle and manifest, embeds them in a fresh installer image, signs
that image with the same certificate, and runs the installer's read-only
`verify` command. It does **not** install, update, roll back, or uninstall
anything. A failure removes only the new local output and any certificate
entries that this invocation itself created.

When preparation succeeds it prints the exact command for the signed installer.
Start that command from a normal, non-elevated PowerShell session or by opening
the signed executable. This preserves the intended native confirmation and
fixed UAC handoff. Do not pass an installer command or an application path.

## Acceptance check

After approving the native confirmation and the Windows UAC prompt:

1. Confirm the installer reports that the signed Anodrel release installed.
2. Open **Anodrel Product Fixture** from the Windows Start menu. This is the
   actual registered Shell Link route, not a direct child command.
3. Confirm the product session displays *Signed child, authenticated window*.
4. Activate **Complete product session**, or reach it with Tab and Enter.
5. Confirm the window and fixture child both exit promptly.
6. From a normal PowerShell session, run the printed signed installer with
   `verify`; it must accept the selected installed release.
7. Close the product window with its title-bar button in a separate run and
   confirm the child also exits.

The prepared image makes the signed installer and its direct policy checks
available; it cannot prove that the native consent, UAC, Start-menu, Explorer,
or window interactions occurred. Record those visible checks for a Windows
release candidate rather than treating successful preparation as acceptance.

## Removal

First, remove the installed fixture from an **elevated** PowerShell session.
The installed fixed signed image, not the original download, is the only
accepted removal command:

~~~powershell
$programFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$uninstaller = Join-Path $programFiles 'Anodrel\Applications\org.anodrel.product-fixture\0.1.0\uninstaller\anodrel-windows-installer.exe'
& $uninstaller uninstall
~~~

That command revalidates the selected release, package version, and publisher
before removing the fixed policy, ordinary Program Files package content, and
derived Start-menu entry. The running signed image and its empty directories
are queued for deletion at the next restart. It accepts no application
identity, package path, registry path, or cleanup target.

After that command succeeds, remove the generated development material from an
elevated PowerShell session:

~~~powershell
.\scripts\prepare-installed-product-fixture.ps1 -Remove
~~~

The script refuses to remove certificate trust or its local output while a
valid product-fixture policy remains selected. This prevents an operator from
leaving an installed fixture whose signature no longer chains to its intended
development trust.

## Relationship to the staged fixture

`provision-product-fixture.ps1` remains useful for the smaller, staged
launcher/session check described in [Development product fixture](PRODUCT_FIXTURE.md).
It writes its own record directly and intentionally creates neither an installer
image nor a Start-menu entry. Remove it before preparing this installed fixture;
likewise, uninstall this fixture before returning to the staged procedure.

The installed fixture does not make the staged fixture's checks redundant. The
staged route is the focused host-launch diagnostic; this route exercises the
signed distribution and registration chain around it.

## What remains out of scope

- Production certificate authority, key custody, renewal, revocation, and
  timestamp service.
- A public installer download, managed deployment, installer progress UI, repair
  flow, localization, or alternative installation scope.
- A live update catalogue, key rotation, update transfer, restart policy, or
  automatic update scheduling.
- A claim that the development certificate or fixture is production-ready.

See [Windows installer contract](WINDOWS_INSTALLER.md), [release image](RELEASE_IMAGE.md),
[signing](SIGNING.md), [product launcher](PRODUCT_LAUNCHER.md), and Decision
0189.
