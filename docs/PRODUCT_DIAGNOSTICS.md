# Anodrel product and update diagnostics

These Windows-only diagnostics exercise verified product-session and update
surfaces. They are distinct from ordinary compiled development samples because
the product fixture changes temporary machine trust and the checks include
visible desktop behaviour.

## Development Windows product fixture

This is the only path that exercises the complete verified launcher and product
session: machine policy, locked digest revalidation, Authenticode publisher
match, child-only bootstrap delivery, authenticated pipe, native window, one
semantic action, and coordinated shutdown.

It is a **development-machine** procedure. Provisioning installs a locally
generated code-signing certificate into the machine root and trusted-publisher
stores and writes one `HKEY_LOCAL_MACHINE` policy record. Both need an elevated
PowerShell session, and both are reversed by `-Remove`. Read
`docs/PRODUCT_FIXTURE.md` before running it.

From an **elevated** PowerShell session at the repository root:

~~~powershell
.\scripts\provision-product-fixture.ps1
~~~

The script builds the fixture, host, and provisioning helper, stages a package
under `%LOCALAPPDATA%\Anodrel\ProductFixture`, creates or reuses the development
certificate, signs both staged executables, installs machine trust, and writes
the record. It ends by reporting that the machine record validates.

To check the current state at any time — including before provisioning anything
— use the query-only switch, which changes nothing and needs no elevation:

~~~powershell
.\scripts\provision-product-fixture.ps1 -Verify
~~~

Then, from an ordinary session:

~~~powershell
& "$env:LOCALAPPDATA\Anodrel\ProductFixture\bin\anodrel-windows-host.exe" --product-launch org.anodrel.product-fixture
~~~

Confirm each of the following:

1. an **Anodrel Product Session** window opens on a host-owned waiting screen;
2. it is replaced by the fixture's document, headed *Signed child, authenticated
   window*;
3. **Complete product session** responds to hover, and Tab plus Enter reaches it;
4. activating it closes the window within a moment — that is the fixture's
   `session.close` reaching the host-owned close signal, not the window manager;
5. the host process exits; and
6. `anodrel-product-fixture.exe` is gone from Task Manager.

Also check the two failure paths. Close the window with its title-bar button
instead of activating the action: the window must close, the host must exit, and
the child must still disappear. Separately, end `anodrel-product-fixture.exe`
from Task Manager while the window is open: the window must close on its own.

There is a third path worth checking from the Startup Lab, because a launch
takes a noticeable moment. Click **Development Fixture** and immediately close
the Startup Lab window, before the product window appears. The host must exit
and `anodrel-product-fixture.exe` must not be left running: a session that
finishes starting after its surface has gone is ended by the host rather than
handed to a window that no longer exists.

The Startup Lab reads the same provisioning state:

~~~powershell
cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --showcase apps/sample/anodrel.application.json
~~~

With the fixture provisioned, **Development Fixture** is drawn live, reads
*Development only, not a product*, responds to hover, and opens a window titled
**Anodrel Development Product Fixture**. Run
`.\scripts\provision-product-fixture.ps1 -Remove` and repeat: the tile must
return to *Not provisioned*, dimmed and marked **PLANNED**, and ignore clicks.

Remove the fixture when you are finished:

~~~powershell
.\scripts\provision-product-fixture.ps1 -Remove
~~~

The protocol half of this path is covered automatically and needs no
provisioning:

~~~text
cargo test --manifest-path native/Cargo.toml -p anodrel-product-fixture
~~~

For the separate signed installer, Program Files, and Start-menu route, follow
[installed development product fixture](INSTALLED_PRODUCT_FIXTURE.md). It
prepares the image but leaves consent, UAC, launch, and removal visible
operator checks.

## Windows taskbar-progress diagnostic

Run the fixed no-trust-change diagnostic described in
[Windows product-update progress](PRODUCT_UPDATE_PROGRESS.md).
