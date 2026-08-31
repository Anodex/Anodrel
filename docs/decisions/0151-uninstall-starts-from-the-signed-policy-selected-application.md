# Decision 0151: Uninstall starts from the signed policy-selected application

**Status:** Accepted

**Date:** 2026-08-31

## Decision

The owned uninstaller first verifies its current signed embedded release, then
loads only the fixed machine record for that release's application identity. It
requires the installed executable's accepted Authenticode fingerprint to match
both the installed record and the embedded release publisher before returning
an opaque uninstall target.

It accepts no application ID, package root, registry path, or executable from a
command line. This preflight deletes nothing. Later record removal and package
tree deletion consume only this opaque verified result.
