# Scripts

This directory contains repeatable development, verification, packaging, and
release helpers.

Scripts must be safe to run from a clean checkout, document their prerequisites,
and avoid writing secrets or generated output into tracked source directories.

## provision-product-fixture.ps1

Provisions or removes the development-only Windows product fixture described in
`docs/PRODUCT_FIXTURE.md`. It builds the fixture and its provisioning helper,
stages a package outside the repository, signs the executable with a locally
generated development certificate, installs that certificate into machine trust,
and writes one machine-policy record.

Provisioning and `-Remove` need an elevated PowerShell session, change machine
certificate trust, and are for development machines only. Run it with `-Remove`
when finished.

`-Verify` reports whether the machine record currently validates. It is a query
only, changes nothing, and needs no elevation.
