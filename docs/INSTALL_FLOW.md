# Windows interactive initial-install flow

**Status:** The fixed no-argument initial-install route is implemented. It
needs a signed end-to-end fixture run before it can be claimed as validated on
a real machine. Progress, restart, repair, and a full installer window remain
separate work.

## Purpose

Running the signed installer with no command starts only one first-install
flow. It accepts no identity, path, package, policy, certificate, endpoint,
directory, command, or elevation argument. The existing named commands remain
for explicit elevated operator routes.

## Fixed sequence

~~~text
signed current installer + missing policy
                    |
                    v
native confirmation (No by default)
                    |
                    v
Windows UAC for fixed `install`
                    |
                    v
wait outside a native UI thread
                    |
                    v
fixed policy and signed-release postcondition proof
~~~

Every stage is required. A declined native confirmation changes nothing. A
declined UAC prompt, failed launch, missing process handle, failed wait, or
nonzero child exit has no completion-proof route. A zero child exit alone is
not claimed as installation success.

The parent writes only a closed outcome to its console. It does not show
progress, parse child output, create an installer window, restart an
application, or report user-visible completion beyond that operator outcome.

## Existing commands

`install`, `update`, `rollback`, and `uninstall` remain direct elevated
operator commands. `verify` remains read-only. The no-argument path refuses a
selected existing policy instead of treating the locally supplied installer as
an update; the separate signed update system owns update discovery and choice.

## Verification boundary

The development test image is deliberately unsigned, so automated tests prove
that it stops before a dialog or UAC request. A positive run requires a signed
resource-bearing installer and a machine-trust decision made by the owner. See
[product fixture](PRODUCT_FIXTURE.md) and [Windows release readiness](WINDOWS_RELEASE.md).

See [Windows installer](WINDOWS_INSTALLER.md), [initial-install consent](INSTALL_CONSENT.md),
[initial-install handoff](INSTALL_HANDOFF.md), and Decision 0180.
