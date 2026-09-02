# Decision 0182: Product registration follows selected machine policy

**Status:** Accepted

**Date:** 2026-09-02

## Context

Windows product surfaces need a display name, publisher label, and executable
target that continue to match the selected release after update or rollback.
The signed installer manifest supplies safe display metadata, but it is not
available to a later host or rollback operation. Holding a target in installer
memory or deriving it from mutable package content would create stale or
untrusted registration.

## Decision

Add record version 1.21. It strictly extends 1.20 with a required `product`
object whose two bounded strings use the signed release metadata grammar. The
installer renders that object only from release-manifest v1.2. The portable
installed-record parser retains it as private native-host data alongside the
already validated package root, executable, and update catalogue source.

No application protocol operation, capability, SDK method, path, registry key,
filename, command, URL, certificate selector, or authorization decision uses
the display text. A later Windows registration adapter must re-read selected
policy rather than retain an installer-time target.

## Consequences

- A later Start-menu or Apps & features adapter can follow update and rollback
  through one selected policy record.
- Release version 1.2 always renders record version 1.21; older releases keep
  their existing record versions and no implicit product surface.
- The portable installed-record contract gains a display-only field without
  changing application grants or process authorization.

## Revisit conditions

Revisit for localized metadata, product icons, operating-system package
identity, multiple channels, a public host metadata API, a machine registration
broker, or another platform.
