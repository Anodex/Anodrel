# Windows verified product sessions

**Status:** Internal Windows-host contract. It is not a public SDK surface.
The development entry points are the `--product-session <applicationId>`
command and the Startup Lab launch tile, which exists only while a machine
record and signed executable currently validate. An installed product Start-menu
entry uses the separately verified `--product-launch <applicationId>` route.
The one application that can be provisioned today is the development fixture in
`docs/PRODUCT_FIXTURE.md`.

## Purpose

A Windows product session binds exactly one machine-policy application record,
verified child process, authenticated named-pipe worker, and grouped native UI
resource set. `anodrel-windows-product-session` is the only composition point
for those lifetimes.

## Start sequence

The host selects a valid application ID, its own host name, and a unique
session ID. On a worker thread it calls `start_registered_product_session`:

1. create the registered interactive pipe and host-owned UI resources;
2. convert the private invitation into the existing child-only bootstrap
   record;
3. launch only the locked, digest-revalidated, signer-verified executable;
4. start one pipe worker and one child-exit watcher; and
5. pass only the returned UI group to the host's internal authenticated-window
   entry point.

An application cannot choose the record, executable, launch arguments, window
title, resource group, stop signal, or process handle. The coordinator adds no
new protocol message or compatibility surface.

## Lifetime rules

The pipe worker is never run on the Win32 UI thread. If the child exits, its
watcher stops pending pipe I/O and signals the native window to close. If the
pipe ends, its worker signals the window and terminates the tracked child.
Explicit shutdown does all three operations as best-effort host cleanup.

The caller must keep `RunningProductSession` alive while it runs the native
window. Ending it does the same work whichever way it happens: `finish` requests
shutdown, joins both workers, and reports a safe host failure category, while
dropping the value requests shutdown and joins both workers without a category.

Both endings are complete because a host may own the session through a native
window rather than through a call stack. If an implicit end only signalled, a
closed product window would leave a pipe worker and an exit watcher running for
the rest of the host's life. Neither join waits on user-paced work: shutdown has
already cancelled pending pipe I/O and terminated the tracked child, so both
workers are returning before the first join begins.

## Host activation

`--product-session <applicationId>` starts the coordinator on a worker, waits
for it, then runs the authenticated window on the host's own UI thread and calls
`finish` when that window returns. The identity selects which already-provisioned
machine record to read; it supplies no record, package, executable, capability,
or child argument.

`--product-launch <applicationId>` is the installed-product route. Before it
starts the same coordinator, it verifies that the current executable is the
selected product launcher and that its locked digest and publisher still match
the selected record. It is generated only by the Start-menu writer; it accepts
no child path, record path, capability, or application-defined argument. See
`docs/PRODUCT_LAUNCHER.md` and Decision 0187.

The Startup Lab tile takes a different ownership route because its message loop
is already running. A click starts the coordinator on a worker; the worker posts
one private window message; the UI thread then creates the product window and
that window's view owns the session. Destroying the window drops the session,
which ends it exactly as `finish` would. Exactly one product session may exist
at a time, and a failed start reports only the tile's existing planned state.

Between those two steps the session waits in a host-owned slot, and a start
takes long enough — machine policy, a locked hash, an Authenticode chain, and
process creation — that the surface can close first. That gap is closed twice:
the worker ends the session itself if its message cannot be posted, and the
host ends any session still waiting once its message loop returns. Both are
required, because a posted message is only delivered while the loop runs, and
because a session left in that slot would never be dropped at all — its
verified child would then outlive the host, which is exactly what this
lifecycle exists to prevent.

## What it still does not do

The current coordinator has no restart, background mode, application-driven
graceful-exit protocol, output capture, public window API, or multi-window
policy. It is not a packaging, installation, or update mechanism, and it makes
no claim of parity with a framework runtime. See Decisions 0020, 0058, 0059,
0060, and 0061.
