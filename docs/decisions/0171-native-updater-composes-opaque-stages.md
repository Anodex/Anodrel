# Decision 0171: Native updater composes opaque stages

**Status:** Accepted

**Date:** 2026-09-01

## Context

Anodrel now has individual direct boundaries for signed catalogue discovery,
private cache recovery, image streaming, locked image acceptance, and UAC
handoff. Letting a host manually wire their raw paths and intermediate values
would make it easier to bypass a necessary check or change the required order.

## Decision

Add one native-only updater crate. It accepts only a host-selected application
ID at discovery. It first opens and recovers the fixed per-application cache,
then retrieves and preflights the signed policy-selected catalogue. Its opaque
offer can be downloaded only into that cache; the resulting image is locked,
re-verified against the same signed candidate, and consumed only by the fixed
Windows UAC `runas update` handoff.

The composition exposes no application protocol, endpoint, path, installer,
argument, version, publisher, registry, certificate, progress, scheduler, or
restart choice. It intentionally stops before user-visible consent and final
installed-policy confirmation.

## Consequences

- Native hosts have one auditable safe ordering instead of recreating it.
- Each earlier adapter retains its narrow responsibility and can be tested in
  isolation.
- Calling discovery still performs one network request, so a future host must
  place it behind explicit user intent before presenting a product surface.
- A production positive acceptance run remains necessary; unit checks cannot
  establish real signing, UAC, installation, or policy publication.

## Revisit conditions

Revisit for a consent protocol, update UI, download progress, restart policy,
automatic scheduling, multi-channel delivery, a privileged broker, another
platform, or a signed end-to-end acceptance runner.
