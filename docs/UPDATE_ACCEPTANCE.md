# Windows update acceptance

**Status:** The post-handoff machine-policy proof and fixed-identity development
acceptance runner are implemented. A real signed fixture run remains required
before this can be claimed as production acceptance.

## Purpose

A successful UAC request or installer process exit is not evidence that a new
release was selected. After a fixed `update` process exits with code zero, the
native updater re-reads the fixed machine record for the candidate's signed
application identity and proves all of these facts:

1. the record and selected package remain valid;
2. Windows accepts the selected executable's Authenticode signature;
3. that signer matches the selected record and the verified candidate; and
4. the selected canonical package directory has the candidate's exact version.

The resulting opaque proof means only that current machine policy selected the
same signed application release that passed the handoff gate. It does not claim
that a process was restarted, that data migrated, that a user observed anything,
or that future launch succeeds under every runtime condition.

## Failure handling

The check does not run when the installer reports a nonzero exit code. Any
record, signature, publisher, or version mismatch is a safe failure and leaves
the updater without acceptance proof. No rollback, delete, repair, retry,
relaunch, notification, or error detail is produced by this boundary.

## Verification still required

`anodrel-product-update-acceptance` is the no-argument operator diagnostic for
the fixed development fixture identity. The eventual signed fixture acceptance
must exercise an actual newer installer, the Anodrel prompt, UAC prompt, the
elevated update transaction, process completion, and this postcondition on a
development machine with its temporary signing certificate trusted. Production
acceptance additionally needs the real certificate custody, timestamping, and
release endpoint decisions.

See [update flow](UPDATE_FLOW.md), [update handoff](UPDATE_HANDOFF.md), and
[Windows installer](WINDOWS_INSTALLER.md), and the
[development product-update fixture](PRODUCT_UPDATE_FIXTURE.md).
