# Decision 0197: The installed uninstaller cleans itself at restart

**Status:** Accepted

**Date:** 2026-09-04

**Supersedes in part:** Decision 0196's package-cleanup location.

## Context

The selected release needs a durable signed removal image for Apps & features.
Windows does not let an executing image delete its own directory. Moving that
image outside the version directory would introduce a second lifecycle that
must remain synchronized across install, update, and rollback, and would still
leave the executing image behind at final removal.

## Decision

Keep the fixed signed installer copy inside the selected version directory at:

~~~text
<selected package root>\uninstaller\anodrel-windows-installer.exe
~~~

The owned uninstall transaction removes every other normal file and directory
in that selected package root immediately. It retains only the executing fixed
image and its required ancestor directory. After it has removed selected
policy, it registers direct `MoveFileExW` deletion operations for the image,
then its now-empty directory, then its now-empty package root at the next
Windows restart. The operations are registered in that order, as required for
Windows to delete a directory only after it is empty.

The scheduled operations use only installer-derived paths after full
selected-policy, version, signer, and current-image proof. No application,
registry, product, network, or caller input supplies a deletion target. A
failure to register any delayed removal is reported as incomplete cleanup; it
does not pretend that final removal occurred.

## Consequences

- Apps & features can point at a version-bound signed image without depending
  on a download, installer framework, helper executable, or general shell
  command.
- Product policy and ordinary application files disappear during the uninstall
  action; the final fixed image, empty directory, and empty version root can
  remain until the next restart.
- A same-version reinstall before that restart may require the documented
  recovery path. A different version does not reuse that retained version
  directory.
- The implementation must prove that all immediate cleanup paths are ordinary
  non-reparse objects and must test the exact delayed-deletion order.

## Revisit conditions

Revisit for a separately signed cleanup helper, a Windows package identity,
per-user installation, an administrator-managed restart policy, repair,
multiple product channels, or another operating system.
