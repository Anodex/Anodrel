# Decision 0189: Development installer fixture keeps release assembly fixed

**Status:** Accepted

**Date:** 2026-09-03

## Context

The development product fixture proves an invited child, verified launcher, and
native session only from a staged directory. The owned Windows installer can
already author an uncompressed bundle, derive a strict release manifest, embed
it in one installer image, sign that image, install it under Program Files,
publish machine policy, and register a Start-menu link. Those components have
not yet had one narrow development path that assembles them as a joined release
chain.

Letting a fixture script accept an application identity, package contents,
certificate selector, output directory, or installation command would turn it
into an unreviewed packaging or machine-administration interface. Automatically
running an elevated installer from a trust-provisioning script would also hide
the native consent and UAC boundary that the installer is meant to prove.

## Decision

Add one Windows-only `prepare-installed-product-fixture.ps1` script. It accepts
only `-Remove`; every build and product value is fixed in the repository. Its
normal route:

1. refuses a currently valid selected record for the fixture identity;
2. builds the fixed first-party fixture child, verified launcher, installer
   shell, and release-authoring tools;
3. stages the fixed fixture package in one known local-development directory;
4. creates or reuses exactly one development code-signing certificate in the
   current user's personal store and adds only that certificate to the local
   machine root and trusted-publisher stores;
5. signs the fixed child and launcher, authors the bounded bundle and strict
   version-1.4 release manifest, embeds both in a fresh installer image, and
   signs one fresh installer copy with that same certificate; and
6. runs the installer's read-only verification before printing the exact
   no-argument installer command.

The script never runs `install`, `update`, `rollback`, or `uninstall` itself.
An operator starts the signed installer normally to see native consent and the
fixed UAC handoff, then launches the registered Start-menu entry. The removal
route refuses while a valid fixture record remains selected, so an operator
must first use the matching signed installer's explicit elevated `uninstall`
command. It then removes only its known generated directory and certificate
entries.

The certificate, package version, update source, product metadata, grants,
child path, launcher path, and output names are development-fixture constants.
No artifact belongs in source control or is a production signing identity.

## Consequences

- The installer, policy record, Program Files payload, Start-menu launcher,
  and removal route have one repeatable, direct-API-only development acceptance
  setup.
- A real machine trust change and the final consent, UAC, Explorer, and window
  checks remain explicit operator work. Automated preparation is not evidence
  that those desktop effects occurred.
- The existing staged fixture remains available for its narrower launcher-only
  check. The two modes cannot coexist under their shared identity, so each
  refuses a selected policy rather than overwriting it.
- Production certificate custody, timestamp policy, hosted update catalogue,
  production installer UX, and release operation remain separate product
  decisions.

## Revisit conditions

Revisit for an isolated-machine acceptance environment, a production signing
identity, timestamping, a user-facing release channel, automated update
acceptance, an installer UI, another installation scope, or another platform.
