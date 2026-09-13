# Windows Apps & features removal flow

**Status:** The fixed native consent, UAC handoff, repeated elevated preflight,
and policy-absence proof are implemented in the installer shell. The Apps &
features registration now invokes the route after its selected-policy proof.

## Purpose

An Apps & features entry must work from a normal Windows user context while
keeping removal authority fixed. It never invokes a downloaded installer,
application child, shell command, user-supplied argument, or application
protocol request.

## Fixed sequence

~~~text
selected signed uninstaller image
    -> selected policy, version, publisher, and image proof
    -> one native Remove / Cancel confirmation
    -> one fixed UAC handoff to the same image's internal uninstall command
    -> elevated removal stages a byte-identical signed helper outside the package
    -> helper repeats proof under per-application maintenance exclusion
    -> private READY / COMMIT / ACCEPTED exchange
    -> fixed policy-absence proof; original uninstallers exit
    -> helper removes package and displays final success or incomplete-cleanup result
~~~

The uninstaller image and the `remove` and `uninstall` route names are Anodrel
constants. The registry entry contains only the quoted selected image path and
the fixed `remove` route. It does not quote or pass product display text,
application identity, package path, registry key, version, publisher, or
network location.

## Boundaries

The first proof does not modify policy, create a process, elevate, display
progress, delete files, or retain a preference. Native cancellation does not
start UAC. The UAC child repeats preflight rather than trusting a normal-user
process. A successful child exit requires the final policy proof, which does
not claim that package cleanup has completed. The helper's separate native
success dialog is shown only after the package directory is removed.

## No-restart cleanup and recovery

The helper derives its target independently from its own signed manifest and
the selected machine record. No path or PID is accepted in its command line or
private four-byte control frames. It retries deletion for at most 30 seconds;
it does not kill the application or schedule reboot deletion. Install, update,
rollback and cleanup share one non-inherited maintenance lock, with refreshed
state checks under that lock.

The helper cannot delete its own running executable. Its signed image remains
in a protected, uniquely named cache outside the package until the next signed
install/update or elevated `cleanup-cache` command. Close the result dialog
before retiring that cache. An empty protected commit marker permits that same
signed recovery route to finish an interrupted package deletion with absent
policy. After that selected package is gone, it can retire only the one lower
package named by the validated private rollback record; it never scans version
directories. Unknown files, invalid signatures, links, active helpers and
invalid policy fail closed. See Decisions 0219 and 0222 for the exact protocol
and retirement rules.

Old installed binaries are unchanged by compiling this code. Previously queued
restart deletions are not cancelled; do not reinstall over a legacy pending
same-version deletion. The new helper path requires a freshly signed install.

## Verification status

Automated tests exercise actual Windows mapped-image locking, process exit,
immediate version-directory reuse, busy-file retry, anonymous-pipe deadlines,
invalid frames, maintenance exclusion and unsigned-image rejection. They use
isolated temporary directories, not machine policy or certificate trust.
Signed install -> remove -> cache retirement -> immediate reinstall, native
consent/UAC, cancellation and interrupted signed recovery still require the
installed-fixture acceptance run; unit tests do not establish those outcomes.

## Exclusions

This flow does not add silent uninstall, repair, custom install UI, arbitrary
arguments, restart control, data deletion, updater service, telemetry,
Application User Model ID, or application control over registration.

See [Apps & features](APPS_AND_FEATURES.md), [Windows installer](WINDOWS_INSTALLER.md),
and Decision 0198.
