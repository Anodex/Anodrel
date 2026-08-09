# Decision 0061: Prove verified product sessions with a development-only signed fixture

**Status:** Accepted

**Date:** 2026-08-08

## Context

Decisions 0017 through 0020 built machine policy, record validation,
Authenticode verification, and locked launch. Decisions 0058 through 0060 built
the registered interactive session, the pipe stop signal, and the product-session
coordinator that joins them. Every one of those components is tested in
isolation, and the joined path has never run, because the repository ships no
signed executable and no machine-provisioned record.

That gap is not cosmetic. Decision 0020 already recorded that "an opt-in signed
and machine-provisioned test fixture is required for a full launch integration
test", and Decision 0060 recorded the same limitation. Until such a fixture
exists, the Startup Lab launch tile cannot be honestly linked, and a defect in
the joined lifetime — an orphaned child, a blocked pipe worker, a window
attached to the wrong session — would only be found by a future product.

The alternatives were worse. Waiting for a real signed product means the first
execution of this path happens under production pressure. Weakening the checks
for a test — accepting an unsigned executable, a current-user policy source, or
an environment-variable override — would delete the boundary the milestone is
supposed to prove.

## Decision

Add a development-only, signed, machine-provisioned Windows product fixture and
two host-only activation routes. The fixture satisfies every existing check
rather than bypassing any of them.

The fixture has three parts:

1. **`anodrel-product-fixture`** — a first-party child executable built from
   Anodrel crates, the Rust standard library, and direct Windows APIs. It reads
   one `ANBI` record from its inherited standard input, connects to the named
   pipe, authenticates, calls `platform.health`, replaces the UI document,
   waits for one host-rendered semantic action through `ui.events.read`, calls
   `session.close`, and exits. It has no arguments, configuration, output, or
   capability beyond `ui.document.write`, `ui.events.read`, and `session.close`.

2. **`anodrel-product-provisioning`** — a first-party development helper, not
   part of the host. Given a staged package root it recomputes the executable
   digest, obtains the accepted Authenticode leaf fingerprint through the
   existing signature adapter, composes the strict record from `docs/LAUNCH.md`,
   validates that record with the same parser the host uses, and writes exactly
   one `record` value under the existing machine policy key. It also removes and
   verifies. It cannot name a hive, key path, or value name.

3. **`scripts/provision-product-fixture.ps1`** — orchestration using Windows
   tooling only: Cargo for the build, `New-SelfSignedCertificate` for the
   development certificate, `Set-AuthenticodeSignature` for signing, the
   `LocalMachine\Root` and `LocalMachine\TrustedPublisher` stores for trust, and
   the helper for the record. Its removal mode reverses every step.

Activation is host-only and has two entry points. `--product-session
<applicationId>` starts the coordinator on a worker thread and runs the
authenticated window on the UI thread with that session's grouped resources. The
Startup Lab resolves its launch tile from a **verification-only** preflight —
machine record, locked digest revalidation, Authenticode, publisher fingerprint
— that creates no process, pipe, or bootstrap material. The tile is drawn and
hit-tested from that single resolved value, so it is inert unless the fixture
currently validates.

The self-signed development certificate is installed into machine trust. That is
a real machine change, is scoped to a development machine, and is reversible
through the same script.

## Consequences

Positive:

- the verified launch, authenticated pipe, native window, action round trip, and
  child cleanup can be exercised as one lifetime for the first time;
- no existing check is relaxed: the fixture is signed, machine-provisioned,
  digest-locked, publisher-matched, and argument-free like any future product;
- the Startup Lab tile finally reflects real provisioning state rather than a
  compile-time constant, and it fails closed on an unprovisioned machine;
- record writing lives in one auditable development helper that the host never
  links, so the host keeps its read-only machine-policy relationship.

Tradeoffs:

- provisioning installs a locally generated code-signing certificate into machine
  trust stores and requires an elevated shell; this is acceptable only on a
  development machine and must be removed afterwards;
- the fixture is not a product: it proves one child, one window, and one action,
  and says nothing about packaging, installation, updates, multi-window policy,
  restart, or background execution;
- the Startup Lab now performs blocking verification work during startup, which
  costs one locked read and one Authenticode evaluation before its window opens.

## Revisit conditions

Revisit when a real signed application is packaged and installed by a documented
installer, when update coordination needs its own lock protocol, when product
sessions gain multi-window or restart policy, or when a non-Windows host needs
an equivalent verification fixture.
