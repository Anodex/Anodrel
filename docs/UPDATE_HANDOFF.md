# Windows elevated update handoff

**Status:** The locked image-acceptance gate and direct UAC launcher are
implemented. A later updater coordinator will choose the cache root, collect
an explicit user decision, wait off the UI thread, and recover retained files.

## Purpose

An update image is not allowed to become an elevated process merely because it
matched an HTTPS digest. The handoff accepts only an opaque image value that
passed all of these checks while Windows held the source file against writes:

1. the attached-CMS catalogue and installed policy selected it;
2. its streamed bytes matched the signed descriptor;
3. Windows accepted its Authenticode signature;
4. its embedded release manifest and bundle were valid; and
5. its identity, version, and publisher exactly matched the signed catalogue.

The locked image is the only input to the direct `runas` launcher. No
application, command line, environment value, URL, file name, argument,
working directory, or installer command is accepted.

## Image acceptance

The native gate opens the freshly created private `.exe` as a Windows resource
image with the system's exclusive-write resource mapping flags. It then reads
the fixed release resources, verifies the complete payload and Windows
Authenticode, and compares the release facts to the already verified catalogue.
The mapping stays alive until the launch attempt completes. This prevents a
different process from opening the image for writing between its checks and the
handoff.

The gate does not run code from the candidate while inspecting resources. It
does not install, elevate, launch, create trust, choose a cache directory, or
report a release path to an application.

## Direct elevation

The launcher calls Windows `ShellExecuteExW` with the explicit `runas` verb,
the checked absolute image path, and the one fixed `update` argument. Windows
shows the normal UAC consent experience; user cancellation is a normal safe
outcome. A process handle is retained so the caller can wait for completion
away from a UI thread.

The elevated image independently repeats its own Authenticode, embedded-release,
publisher-continuity, version, staging, promotion, and machine-policy checks.
The original process cannot treat a successful launch request or an exit code as
proof that a release was installed.

If the caller abandons a still-running process, the private file is retained
for the later owned cache-recovery boundary instead of being deleted while it
is executing. There is no background service, scheduled task, automatic retry,
or cleanup scan in this slice.

## Exclusions

This boundary does not select update endpoints, choose a cache root, show an
Anodrel dialog, request consent on its own, run a progress UI, schedule work,
restart an application, interpret installer output, or recover a retained
image. It does not replace Windows signing, UAC, or the elevated installer
transaction.

See [update discovery](UPDATE_DISCOVERY.md),
[update delivery](UPDATE_DELIVERY.md), and
[Windows installer](WINDOWS_INSTALLER.md).
