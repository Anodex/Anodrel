# Local signed update fixture

**Status:** The fixed local publication, listener, release preparation, and
read-only selected-release verifier are implemented. A real operator acceptance
run remains required.

## Purpose

The existing fixed development update-acceptance runner proves the client-side
sequence only when an operator has independently provided a newer signed image
and HTTPS catalogue. This fixture will supply those two artifacts locally so a
development machine can exercise the complete owned path without a public
endpoint, a downloaded server, or a configurable update source.

It is a development acceptance fixture, not a product update service, release
channel, web server, SDK feature, or application capability.

## Fixed scope

The fixture will use one identity distinct from the ordinary product and
no-restart fixtures: `org.anodrel.local-update-fixture`. Its initial signed
release will select only this update catalogue location:

~~~text
https://localhost:45863/anodrel/local-update/stable.p7s
~~~

The catalogue will describe only one newer signed 0.1.1 installer at:

~~~text
https://localhost:45863/anodrel/local-update/releases/0.1.1/installer.exe
~~~

The local publisher certificate signs the installed 0.1.0 image, the 0.1.1
candidate image, and the attached CMS catalogue. A separate local server TLS
certificate identifies `localhost`; it is transport trust only and cannot
substitute for the publisher-continuity or CMS checks.

## Publication boundary

An Anodrel-owned native fixture server will use the Windows HTTP Server API.
It will bind only the fixed HTTPS prefix after the elevated fixture-preparation
step has installed a matching localhost TLS binding and a URL reservation for
the current operator. The server accepts no command-line arguments or
configuration files and reads only its fixed private fixture root.

It serves exact `GET` requests for the catalogue and installer paths above. It
returns a fixed not-found response for every other method or path, accepts no
request body, writes no application data, proxies no traffic, and exposes no
directory listing, logging endpoint, shutdown endpoint, or mutable control
surface. Before accepting a request, startup independently checks both regular
files under its fixed root, locks and verifies the candidate's Authenticode and
embedded release, then verifies that the attached CMS catalogue has the same
publisher, identity, version, candidate-release facts, byte length, and fixed
HTTPS installer location. The locked candidate remains held while the server
runs.

The preparation script will create the two fresh signed installers, derive and
sign the catalogue from the locked 0.1.1 image, configure the temporary
Windows-only TLS endpoint, and register only the fixed 0.1.0 release. It will
not install the product or start the server. Normal-user commands will start
the server, invoke the separate no-argument local-fixture acceptance runner,
and stop the server. Removing the fixture will first require policy/package/cache absence,
then remove only this fixture's URL reservation, TLS binding, certificates,
and private staging directory.

After the signed uninstaller has removed the selected policy, the elevated
`-Remove` preparation command builds one fresh signed 0.1.1 retirement image
from the fixture's existing publisher identity and invokes only its fixed
`cleanup-cache` route. That route can retire an exited helper cache and the one
validated lower rollback package retained by the update. It removes neither a
selected policy nor a live package, and it fails closed rather than removing
trust while recovery remains incomplete. The retirement image is private
fixture output, never installed or published.

## Acceptance sequence

~~~text
elevated preparation
    -> installed fixed 0.1.0 release
    -> fixed local HTTP Server API publishes signed catalogue + 0.1.1 image
    -> native Anodrel consent (No by default)
    -> direct WinHTTP HTTPS retrieval and CMS verification
    -> locked image acceptance and fixed UAC update handoff
    -> selected 0.1.1 policy proof
    -> server stop, cache retirement, and fixture removal
~~~

The operator must separately observe the native consent, UAC, and final native
result. Read-only verifiers may prove only fixed policy, package, registration,
and release-version facts; they cannot claim a dialog was seen or a restart was
unnecessary.

## Operator procedure

Run preparation from an **elevated PowerShell**. It changes temporary machine
certificate trust and the fixed loopback HTTP Server configuration, but does
not install the fixture:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
.\scripts\prepare-local-update-fixture.ps1
~~~

Before installing, use this normal PowerShell read-only check to confirm the
prepared artifacts, temporary certificate trust, loopback TLS bindings, and URL
reservation. It does not open a listener or change any state:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
.\scripts\verify-local-update-fixture-preparation.ps1
~~~

In a normal PowerShell window, install the fixed initial 0.1.0 release through
its native consent and UAC route:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
$installer = Join-Path $env:LOCALAPPDATA 'Anodrel\LocalUpdateFixture\AnodrelDevelopmentLocalUpdateFixtureInstaller.exe'
& $installer
~~~

Launch **Anodrel Local Update Fixture** once from the Start menu and close its
product window. Keep the following server command running in a second normal
PowerShell window; it has no network route beyond the fixed localhost HTTPS
prefix, and `Ctrl+C` stops it:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
cargo run --release --manifest-path native\Cargo.toml -p anodrel-local-update-fixture-server
~~~

In another normal PowerShell window, start the fixed acceptance command. First
decline the native Anodrel confirmation once to prove it stops before download;
run it again, approve both the Anodrel and UAC confirmations, and wait for the
success result:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
cargo run --release --manifest-path native\Cargo.toml -p anodrel-product-update-acceptance --bin anodrel-local-update-fixture-acceptance
~~~

Stop the server with `Ctrl+C`, then read the resulting selected release. This
changes nothing:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
.\scripts\verify-local-update-fixture.ps1
~~~

To remove the test fixture, use a normal PowerShell window for the signed
uninstaller, accepting its native remove confirmation and UAC prompt. Close the
helper's final result dialog:

~~~powershell
$programFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$uninstaller = Join-Path $programFiles 'Anodrel\Applications\org.anodrel.local-update-fixture\0.1.1\uninstaller\anodrel-windows-installer.exe'
& $uninstaller remove
~~~

Finally, use an **elevated PowerShell** to retire any exited helper cache and
remove only this fixture's loopback binding, trust, certificates, and local
artifacts. No Windows restart is required by this fresh fixture route:

~~~powershell
Set-Location -LiteralPath 'C:\Users\Owner\Desktop\Platform X'
.\scripts\prepare-local-update-fixture.ps1 -Remove
~~~

## Exclusions

This fixture does not add a public port, arbitrary listener, HTTP, proxy,
certificate bypass, redirects, application-selected endpoint, automatic update,
background service, scheduler, external hosting, installer repair, production
identity, timestamp, or release operation.

See [product-update fixture](PRODUCT_UPDATE_FIXTURE.md),
[update acceptance](UPDATE_ACCEPTANCE.md), and Decision 0221.
