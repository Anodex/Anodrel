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
