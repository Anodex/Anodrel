# Decision 0153: Uninstall cleans only the policy-removed package tree

**Status:** Accepted

**Date:** 2026-08-31

## Decision

Package cleanup consumes only the opaque result of fixed policy-record removal.
It invokes the existing direct normal-tree remover on the preflight-validated
package root, refusing reparse points and never accepting a directory from a
command line. It removes neither application data nor credentials.

If cleanup fails after policy removal, the package remains unselected recovery
material. Retrying the cleanup requires a later recovery route; the uninstaller
does not recreate policy or delete unrelated directories.
