# Anodrel Windows instance lifecycle

**Status:** Windows foundation contract. This is host lifecycle behavior, not a
public application protocol or an executable-trust mechanism.

## Purpose

Anodrel must not let two copies of the same host-owned application surface race
to create competing top-level windows. The first lifecycle slice scopes one
active `--application` window to the validated `applicationId`. The Startup
Lab uses a separate diagnostic scope so it remains independently runnable.

The mechanism uses only direct Win32 named kernel objects and a registered
window message. It carries no command-line arguments, URLs, file paths,
requests, credentials, or application data.

## Contract

After manifest validation and before a host window is created, the Windows host
claims one instance name derived from:

- the fixed `Anodrel.Instance.v1` namespace;
- the validated application ID; and
- a host-controlled scope: `application` or `startup-lab`.

The names use the Win32 `Local\\` namespace, so they are scoped to the current
Windows terminal session. The primary process owns a mutex for its lifetime and
a manual-reset readiness event. It signals readiness only after
`CreateWindowExW` succeeds.

If a second process finds the mutex, it does not create a window. It waits for
the primary's readiness event for at most **one second**, then broadcasts the
matching registered activation message. The existing Anodrel window requests
restore and foreground activation through User32. Windows focus policy remains
authoritative, so this is a best-effort focus request rather than a guarantee.

The second process exits after a successful broadcast. If a primary does not
become ready inside the bound, or the activation request cannot be posted, the
second process returns a safe failure rather than creating another window.

## Boundaries and security

The instance key comes only from the validated manifest. A caller cannot supply
arbitrary activation payload, window handle, or application identity through a
public protocol message.

This is coordination, not authentication. Another process in the same Windows
session can attempt to reserve the known mutex or send the registered message;
the only possible effect is local availability or a best-effort foreground
request. The mechanism grants no capability, exposes no data, and does not
prove executable identity. Signed packages and verified executable launch remain
required before Anodrel accepts a product application process.

## Current lifecycle

~~~text
validated manifest
        |
        v
claim current-session instance
        |
        +-- existing --> wait (at most 1 second) --> activation broadcast --> exit
        |
        `-- primary --> create owned window --> signal ready --> message loop --> release
~~~

The host has no public second-instance event, command forwarding, relaunch
policy, restart manager, or cross-user behavior yet. Those require a verified
application executable and a separately versioned lifecycle protocol.

## Manual verification

From the repository root, run `start.bat`, then run it again while the first
**Anodrel Startup Lab** window remains open. Confirm that only one Startup Lab
window exists and the existing one remains available. Close it, then run
`start.bat` again and confirm a fresh primary window opens.
