# Decision 0152: Uninstall removes only the fixed policy value first

**Status:** Accepted

**Date:** 2026-08-31

## Decision

After signed uninstall preflight, the owned uninstaller removes only the
`record` value from the fixed 64-bit machine key for the verified application.
It does not delete the key, enumerate other values, remove a directory, or
accept an identity or registry location from a caller. Later package cleanup
must consume only the opaque policy-removed result.
