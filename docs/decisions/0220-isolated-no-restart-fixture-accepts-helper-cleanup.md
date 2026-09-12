# Decision 0220: Isolated fixture accepts signed no-restart cleanup

**Status:** Accepted

**Date:** 2026-09-12

## Context

The regular development fixture may already be installed by an older installer
that uses restart-delayed deletion. Rebuilding repository source cannot replace
that installed image or cancel its Windows deletion queue. Reusing the regular
identity to test the new helper would either require a restart or disturb
evidence for the existing fixture.

A general fixture script that accepts an identity, path, certificate, content,
or installer command would become an unsafe packaging interface. The no-restart
test must prove an independent complete signed chain without such authority.

## Decision

`prepare-installed-product-fixture.ps1 -NoRestartAcceptance` selects one second
compile-time fixture package: `org.anodrel.no-restart-fixture`. The native
provisioning tool stages it only through its fixed `stage-no-restart` command;
there is deliberately no matching provision, policy-write, registry-delete, or
argument-selected profile command. Its package content, child path, launcher,
capabilities, version, product metadata, local output root, and installer name
remain fixed Anodrel values.

The mode uses `CN=Anodrel Development No-Restart Fixture`, separate from the
regular fixture certificate. The signed image alone performs machine
installation/removal through its existing consent and UAC routes. The verifier
and `-Remove` route operate only on the fixed second identity. They require
policy/package absence and verified cache retirement before certificate removal.

The acceptance sequence is install, Start-menu launch, session completion,
verification, interactive removal, helper final result, cache retirement, and
immediate same-version reinstall—with no Windows restart. Busy-process,
cancellation, interrupted-cleanup, altered-cache, and legacy-queue cases remain
separate negative checks. Automated tests cover native staging and helper
lifetimes but do not claim visible desktop or trust outcomes.

## Consequences

- The new cleanup path can be verified without changing the older regular
  fixture or its certificate trust.
- The repository contains two fixed development profiles, not a configurable
  fixture framework.
- The no-restart fixture is development-only and provides no production
  distribution, certificate, update, or application capability.

## Revisit conditions

Revisit for a production signing identity, a disposable VM-based acceptance
environment, a different installation scope, more than one supported release
channel, or any request for user-configurable fixture inputs.
