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

## Isolated no-restart acceptance fixture

`-NoRestartAcceptance` selects a second fixed fixture for proving the signed
helper removal route. It has a distinct app identity, display name, local output
directory, and development certificate, so it does not touch an already
installed regular fixture.

| Item | Value |
| --- | --- |
| Application ID | `org.anodrel.no-restart-fixture` |
| Display / Start-menu name | `Anodrel No-Restart Fixture` |
| Local output | `%LOCALAPPDATA%\Anodrel\InstalledNoRestartFixture` |
| Certificate | `CN=Anodrel Development No-Restart Fixture` |

It is not a distribution channel or an application option. It is an isolated,
repeatable Windows release check. The script still accepts no paths, package,
identity, certificate, or installer-command input.

## Prepare the signed installer

This is an explicit development-machine trust change. The script creates or
reuses one fixed Windows legacy-CSP RSA signature certificate in the current
user's personal certificate store and adds that exact certificate to
`LocalMachine\Root` and `LocalMachine\TrustedPublisher`. The direct owned
`SignerSignEx` route requires a certificate associated with a CSP; a default
CNG development key is deliberately replaced rather than reused. It needs an
elevated PowerShell session and reverses those entries during removal.

Before using it, make sure no product-fixture policy record remains. If you
previously ran `provision-product-fixture.ps1`, remove that fixture first; the
two procedures share an identity and an initial installation must refuse an
existing policy. The preparation script also refuses a record that fails
validation; it will not remove development trust or assemble another fixture
over uncertain machine state.

From an **elevated** PowerShell session at the repository root:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
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

### No-restart acceptance procedure

This procedure does not alter the regular fixture:

1. From an **elevated** PowerShell session, prepare the isolated image:

   ~~~powershell
   Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
   .\scripts\prepare-installed-product-fixture.ps1 -NoRestartAcceptance
   ~~~

2. From normal PowerShell, run the printed installer with no arguments; accept
   native consent and UAC. Launch **Anodrel No-Restart Fixture**, complete the
   product session, and close it.
3. Verify the registered package without changing it:

   ~~~powershell
   Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
   .\scripts\verify-installed-product-fixture.ps1 -NoRestartAcceptance
   ~~~

4. From normal PowerShell, run its installed `remove` route. Accept consent and
   UAC, wait for **No Windows restart is required**, then close that dialog.
5. Before restarting, prove that the registered surfaces and package have gone:

   ~~~powershell
   Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
   .\scripts\verify-no-restart-fixture-removal.ps1
   ~~~

   This is read-only evidence of removal. It does not claim to have observed
   native consent, UAC, the helper dialog, a reboot, or cache retirement.
6. Without restarting, from an elevated PowerShell session retire the signed
   helper cache and separate development certificate:

   ~~~powershell
   Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
   .\scripts\prepare-installed-product-fixture.ps1 -NoRestartAcceptance -Remove
   ~~~

7. Prepare and install the same fixture again. That proves immediate
   same-version reuse. Cancellation, a busy application, and interrupted
   cleanup are separate required negative checks.

## Acceptance check

After approving the native confirmation and the Windows UAC prompt:

1. Confirm the installer reports that the signed Anodrel release installed.
2. Open **Anodrel Product Fixture** from the Windows Start menu. This is the
   actual registered Shell Link route, not a direct child command.
3. Confirm the product session displays *Signed child, authenticated window*.
4. Activate **Complete product session**, or reach it with Tab and Enter.
5. Confirm the window and fixture child both exit promptly.
6. From a normal PowerShell session, run the printed signed installer with
   `verify`; it must accept that image's embedded signed release (not prove
   installed machine policy).
7. Confirm **Anodrel Product Fixture** appears in Windows **Installed apps**
   with the signed display name, publisher, and version.
8. Close the product window with its title-bar button in a separate run and
   confirm the child also exits.

After installation, the following read-only command checks the prepared signed
image, direct Installed Apps registration, and exact Start-menu launcher
target. It does not replace the visible Windows checks above:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
.\scripts\verify-installed-product-fixture.ps1
~~~

The prepared image makes the signed installer and its direct policy checks
available; it cannot prove that the native consent, UAC, Start-menu, Explorer,
or window interactions occurred. Record those visible checks for a Windows
release candidate rather than treating successful preparation as acceptance.

### Recorded development acceptance

On 2026-09-11, an operator completed the positive primary path on the
development machine: preparation and read-only image verification, native
consent and UAC installation, Start-menu launch, the visible *Signed child,
authenticated window* session, and **Complete product session** shutdown. This
is evidence for that positive path only. The separate installed-apps,
title-bar-close, installer-verify-after-install, uninstall, helper cleanup,
and recovery checks remain required before a Windows release candidate can
claim full fixture acceptance.

On the same date, a fresh installed fixture passed
`verify-installed-product-fixture.ps1`: it verified the prepared signed image,
the direct `Anodrel.org.anodrel.product-fixture` Installed Apps key, and the
registered Start-menu shortcut's launcher, working directory, fixed host-owned
brand icon, and fixed
product-launch argument. The title-bar-close, uninstall, cleanup-handoff, and
recovery checks remain open.

On 2026-09-12, a second fresh fixture exercised the branded registration and
removal route on the development machine. The read-only verifier reported
`Status: verified` and proved that **Anodrel Product Fixture** targets the
verified launcher, uses the selected package root as its working directory,
passes `--product-launch org.anodrel.product-fixture`, and uses
`C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Anodrel\Anodrel.ico`
at index `0`. Windows Start search visibly displayed the Anodrel mark for that
entry, and launching the registered Shell Link started the verified host and
fixture child.

The signed normal removal route then completed. Before a Windows restart, the
elevated fixture-removal command retired its local output and development trust;
a final read-only machine audit found no selected record, package, shortcut,
installer process, or fixture development certificate. The shared
`Anodrel.ico` remained by design because another Anodrel Start-menu entry may
use it. This records the positive verifier, visible icon, removal, and
no-restart cleanup postconditions; title-bar-close, recovery, cancellation,
busy-application, and interrupted-cleanup checks remain separate evidence.

## Removal

First, remove the installed fixture from a normal PowerShell session. The
installed fixed signed image, not the original download, is the only accepted
removal command. It will show native confirmation and then Windows' UAC prompt:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
$programFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$uninstaller = Join-Path $programFiles 'Anodrel\Applications\org.anodrel.product-fixture\0.1.0\uninstaller\anodrel-windows-installer.exe'
& $uninstaller remove
~~~

That command revalidates the selected release, package version, and publisher
before removing the fixed policy, ordinary Program Files package content, and
derived Start-menu entry. A separate verified signed helper waits for the
original uninstallers to exit, then removes the package. It accepts no application
identity, package path, registry path, or cleanup target.

Wait for the helper's **No Windows restart is required** success result and
close that dialog. A console message saying removal was accepted is not final
package-deletion proof. On incomplete cleanup, close the application and dialog,
then use the elevated cleanup below; it can resume committed signed cleanup.

**Legacy exception:** an already installed old uninstaller still schedules
reboot deletion. Rebuilding this repository does not update that installed
image or cancel previously scheduled deletions. Do not reinstall the same
version over a legacy pending-deletion queue.

Remove the generated development material from an **elevated** PowerShell:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
.\scripts\prepare-installed-product-fixture.ps1 -Remove
~~~

The script refuses to remove certificate trust or its local output while any
product-fixture policy record, installed package directory, or cleanup cache remains, including
a record that fails validation. This prevents an operator from leaving an
installed fixture whose signature no longer chains to its intended development
trust or discarding the evidence needed to investigate a failed machine
transaction.

Before removing trust, the script invokes the prepared signed image's fixed
`cleanup-cache` command when caches exist. Windows must accept the cached
signatures, and every cache must be reclaimed. A running result dialog, invalid
image, unexpected cache contents, or locked package stops the operation without
removing trust. Successful helper cleanup permits preparing and installing the
same fixture again immediately, without restarting Windows.

### Recorded no-restart acceptance

On 2026-09-12, the isolated fixture completed the signed no-restart route on
the development machine. A first preparation and normal installation passed
`verify-installed-product-fixture.ps1 -NoRestartAcceptance`; normal signed
removal then passed `verify-no-restart-fixture-removal.ps1` before a restart.
Elevated `-Remove` retired its helper cache, local output, and development
trust. A second fresh preparation then installed the same `0.1.0` fixture
immediately and passed the installed-fixture verifier again. A final normal
removal, removal verifier, and elevated cleanup all succeeded without a
restart.

The final audit found the versioned package directory absent, no selected
record, Installed Apps registration, Start-menu shortcut, local fixture output,
installer process, or development certificate. The empty application-identity
parent and its maintenance lock may remain; neither is a selected package and
the documented cleanup contract deliberately does not delete it by an
unverified path. This automated process and postcondition evidence does not
claim to have observed the native consent, UAC, or helper dialog.

Cancellation, a busy application, interrupted cleanup, and refusal of altered
cache content remain separate negative checks. Do not remove trust to simulate
a signature failure on an installed fixture; use a disposable development
environment for negative trust tests.

For this fixed fixture, that package directory is the selected version path:
`C:\Program Files\Anodrel\Applications\org.anodrel.product-fixture\0.1.0`.
An empty application-identity parent directory is not a selected package and
does not block development-fixture cleanup.

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
