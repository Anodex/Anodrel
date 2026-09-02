# Windows update cache

**Status:** The fixed cache-root and private-image recovery contracts are
implemented and compose with discovery, download, native consent, and UAC
handoff. A later user-facing host action will collect progress and completion
information.

## Purpose

An updater needs a disposable location for one downloaded installer without
letting an application, command line, environment variable, catalogue, or URL
choose a filesystem path. The direct cache adapter selects only:

~~~text
%LOCALAPPDATA%\Anodrel\Applications\<installed application ID>\cache\updates
~~~

The application ID comes from a fixed machine record selected by native host
composition. The existing Windows known-folder and portable path adapters
derive the parent. The cache adapter creates and accepts only ordinary
directories, rejecting links and junctions at each owned component.

## Image lifetime

The downloader creates only fresh names in this directory:

~~~text
.anodrel-update-<process-id>-<sequence>.exe
~~~

The file is removed when its normal host owner drops. If a UAC handoff may
still have the image open, it is retained rather than deleted. The direct
recovery scanner enumerates only the fixed cache directory and deletes only
normal non-link files with that exact numeric name grammar. It does not descend
into a directory, follow a reparse point, remove another cache item, force a
delete, or treat a failed delete as a success.

An executing installer remains locked by Windows; a recovery attempt that
cannot delete it leaves it in place for another run. A successful deletion is
only cache cleanup, never proof of installation or process completion.

## Exclusions

This is not an application filesystem capability, a general cache API, a
download API, a persistent update queue, a scheduler, a background service, or
a progress store. It does not start an update, elevate, launch a process,
inspect an installer, or restart an application.

See [paths](PATHS.md), [update delivery](UPDATE_DELIVERY.md), and
[update handoff](UPDATE_HANDOFF.md).
