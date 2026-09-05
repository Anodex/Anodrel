# Decision 0199: Product updates start from a native system-menu action

**Status:** Accepted

**Date:** 2026-09-04

## Context

Anodrel's Windows update path can already recover its private cache, discover a
signed policy-selected catalogue, obtain native consent, download and lock one
checked installer, use fixed UAC elevation, and prove the final selected policy.
It deliberately has no application protocol operation: a capability an
application could call would let it perform background update checks and prompt
the person without a native user action. That would violate the explicit-intent
boundary in Decision 0173.

The verified product window is the one native surface that already represents a
specific installed application. It can offer an Anodrel-owned action without
letting application content choose a release endpoint, prompt copy, download,
installer, command, progress value, or restart behaviour.

## Decision

When its selected signed machine record contains an update-catalogue location,
an Anodrel product window adds one fixed `Check for Anodrel updates` item to
its native Windows system menu. The item is absent for older records with no
signed update source and for every diagnostic, development, secondary, or
unregistered window.

Only a local click on that item begins discovery. The host retains the already
validated application identity and performs discovery, download, UAC handoff,
waiting, and postcondition proof outside its UI thread. At most one owned
update attempt runs for the window at a time; its menu item is unavailable
while the attempt is active.

The existing native consent prompt remains the only approval before download
and elevation. After policy proof, the host shows one fixed completion message
that tells the person to restart the application. It does not close, restart,
or relaunch anything. Declines are ordinary terminal outcomes. A discovery,
transfer, handoff, observation, or proof failure produces only a fixed native
failure message, never a path, URL, certificate, native status, installer
output, or application-provided text.

There is no `update.*` protocol operation, capability grant, application menu
item, tray command, timer, startup check, schedule, preference, notification,
release-note view, application-visible progress percentage, or background
update service in this slice.

## Consequences

- A person can initiate an installed product's secure update flow through a
  real native interaction while applications retain no update authority.
- The product host must keep discovery and later blocking work off its UI
  thread, and must keep UI presentation on that thread.
- Completion communicates that the new selected release is ready, but restart
  policy remains a distinct product and lifecycle decision.
- The action remains unavailable until a signed release has selected a real
  update source; no development or mutable configuration can add it.

## Revisit conditions

Revisit for an owned restart coordinator, a bounded native progress surface,
application-independent update settings, release notes, localization,
multi-window ownership, a tray action, automatic scheduling, endpoint/key
rotation, another platform, or production signed acceptance.

## Later amendment

Decision 0200 adds the bounded native progress surface anticipated here: a
fixed host caption and, after Windows declares the taskbar button ready, a
best-effort direct taskbar indicator. It remains outside every application
surface and does not alter this action's explicit-user-intent boundary.
