# Decision 0158: Owned installer commands require an already elevated token

**Status:** Accepted

**Date:** 2026-08-31

## Context

The owned install, update, and uninstall transactions can create Program Files
directories or change the fixed machine policy record. Their library APIs take
no untrusted path or policy data, but a command-line entry point must also make
the operator authority boundary explicit.

## Decision

The first-party command-line tool exposes only `verify`, `install`, `update`,
and `uninstall`, plus the existing development-only `validate-manifest` helper.
Before any machine-changing command it opens the current process token with
query access and reads its `TokenElevation` value through direct Windows APIs.
It proceeds only when that value is nonzero.

The executable never attempts to self-elevate, relaunch, prompt for
credentials, accept an elevated helper path, or transfer its current signed
release to another process. The operator must start the signed installer from
an elevated shell. `verify` and `validate-manifest` remain read-only.

## Consequences

- The elevation check is a clear operator guard, not a substitute for the
  signature, staging, publisher, or policy boundaries.
- UAC presentation and consent occur outside the executable's command parser;
  there is no hidden second process or mutable launch argument surface.
- Every machine-changing command still selects only its current embedded signed
  identity and fixed machine policy location.

## Revisit conditions

Revisit for an approved graphical installer UX, a brokered deployment service,
or a platform package that owns elevation and policy publication.
