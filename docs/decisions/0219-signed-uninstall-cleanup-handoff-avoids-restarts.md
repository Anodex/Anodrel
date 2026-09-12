# Decision 0219: Signed uninstall cleanup uses a private verified handoff

**Status:** Accepted

**Date:** 2026-09-11

**Supersedes:** Decision 0197's restart-delayed final cleanup.

## Context

The original fixed uninstaller retains its own running signed image and asks
Windows to delete that image and its empty package directory at restart. That
is safe, but makes an ordinary uninstall require a reboot before a same-version
development install or fixture-trust cleanup can proceed.

An external cleanup process must not become a general deletion utility. In
particular, its command line, environment, current directory, registry input,
or arbitrary file cannot choose a target. The selected policy is intentionally
removed during uninstall, so a later process cannot use it as its only proof.

## Decision

The signed installer adds a fixed `cleanup` mode and private anonymous pipes.
The elevated selected uninstaller creates a byte-identical signed copy under
the fixed application's Program Files root, outside its version directory.
The stage name is `.anodrel-cleanup-<pid>-<timestamp>`; neither number selects a
process or package. The image has the fixed installed-uninstaller filename.
No target crosses the handoff: the helper derives identity and version from
its own signed manifest and independently reads the still-selected policy.

Before the elevated parent removes selected policy or package content, the
helper independently reads the still-selected machine record and proves:

- its own signed manifest matches that exact selected package;
- the selected executable and both signed installer images have the selected
  publisher; and
- the selected package and uninstaller paths are canonical, ordinary,
  installer-derived locations.

The private protocol has exactly four-byte version-1 frames: helper READY
`ACR1`, parent COMMIT `ACC1`, helper ACCEPTED `ACA1`. READY follows verification;
COMMIT transfers transaction responsibility to the helper; ACCEPTED follows
registration and policy removal, not package deletion. Reads have 30-second
deadlines. Missing or invalid COMMIT makes no policy/package change. A parent
failure after COMMIT must not terminate committed cleanup. No application IPC
field or public capability is added; other frame versions are rejected.

Install, update, rollback, helper cleanup, and cache retirement take the same
exclusive, non-inherited `.anodrel-maintenance.lock` file handle in the fixed
identity root. State is revalidated under the lock. The name remains after
release so concurrent processes cannot acquire different lock objects.

After COMMIT, the helper removes the derived registrations, flushes an empty
`committed` marker in its protected cache, then removes selected policy. It
retries normal package deletion for at most 30 seconds while the original
uninstallers exit. It refuses reparse points and any reappearing selected policy.
It never terminates the application or schedules reboot deletion. Only complete
package deletion earns the native success dialog; timeout reports incomplete
cleanup and preserves the recovery cache.

Windows also prevents deleting the helper's own mapped executable. Therefore
the helper exits leaving its signed cached image, not a package requiring a
restart. This corrects the original design's unverified self-deletion promise.
The next signed install/update, or elevated fixed `cleanup-cache` command,
reclaims exited caches. It verifies the cached image's signature, manifest,
application and publisher, and accepts only the fixed image and optional empty
marker. It never recurses through unexpected cache contents or deletes a live
helper. A pending marker with absent policy permits finishing only the cached
manifest's fixed version tree; with selected policy it never deletes a package.
Maintenance exclusion prevents an old recovery from deleting a new install.
The marker is not a path, command or untrusted capability; administrators remain
inside the machine-trust boundary. No more than 16 caches may be staged.

Fixture cleanup retires these caches before removing development trust and
fails closed on a live, invalid or unreclaimed cache. Legacy installers that
already queued reboot deletion are not migrated or silently cancelled here.

## Consequences

- Ordinary successful uninstall can complete without a Windows restart.
- The helper is first-party, signed, bounded, and independently verified.
- The release path gains a second signed image copy and a small private
  handoff protocol, both requiring unit and installed-fixture acceptance tests.
- Restart-delayed deletion remains unavailable as a silent fallback; a failure
  stays visible for recovery rather than leaving ambiguous cleanup state.

## Revisit conditions

Revisit for a separately versioned helper binary, a Windows package identity,
a machine service, per-user installation, multiple channels, repair, or
another operating system.
