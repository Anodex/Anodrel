# Windows verified product sessions

**Status:** Internal Windows-host contract. No provisioned signed application
fixture ships yet, so this is not a Startup Lab command or public SDK surface.

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
window, then call `finish` after the window returns. Dropping it also requests
shutdown, so an error path cannot intentionally orphan the child. `finish`
joins both workers and reports only safe host failure categories.

The current coordinator has no restart, background mode, application-driven
graceful-exit protocol, output capture, public window API, or multi-window
policy. See Decisions 0020, 0058, 0059, and 0060.
