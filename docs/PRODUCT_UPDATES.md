# Windows product update action

**Status:** The native, user-initiated product action is implemented. It needs
a signed installed fixture for desktop acceptance and production signing
identity before it can be a shipped release feature.

## Purpose

An installed Anodrel product must be able to use the owned update pipeline
without allowing its application code to choose when to check, what to fetch,
or how to install. The product window therefore exposes one Anodrel-owned
Windows system-menu action rather than a protocol operation or application menu
item.

## Availability and action

The native system menu contains **Check for Anodrel updates** only when all of
these are true:

1. the window belongs to a verified authenticated product session;
2. it is that session's primary product window; and
3. the selected signed machine record contains an `updateCatalogue` location.

The action is absent from diagnostics, the Startup Lab, development sessions,
secondary views, older records, and records without a signed update source. It
is not rendered by the application, defined in an application menu model,
exposed through an SDK, carried over IPC, selectable by command line, or
available to a tray callback.

A local system-menu choice starts one check. While its worker is active, the
item is disabled. A further native click cannot start a second discovery,
download, UAC request, or installation attempt for that window.

## Owned sequence

~~~text
native system-menu click
        |
        v
signed-policy discovery on an owned worker
        |
        v
existing native consent on the UI thread (No by default)
        |
        v
private download, checked image, UAC, wait, and policy proof on a worker
        |
        v
fixed native restart-needed message only after policy proof
~~~

Discovery, transfer, UAC waiting, and postcondition proof never run on the
product window's UI thread. The existing consent prompt is the one intentional
modal UI step: it appears only after a signed candidate has been discovered and
shows only its signed version. UAC remains a separate Windows decision.

The host-owned native caption reports checking, downloading at a signed-byte
whole percentage, and installation activity. When Windows has created its
taskbar button, that button mirrors the activity or percentage best effort.
It never reports a speed, remaining time, path, endpoint, certificate,
installer output, or application-supplied value. See [product update
progress](PRODUCT_UPDATE_PROGRESS.md).

The final message says that the update was installed and that the application
must be restarted to use it. It does **not** close, terminate, restart, or
relaunch the application, apply data migration, expose installer output, or
claim that the new process has started. A cancelled native consent or UAC
prompt quietly ends the attempt. Any other failure shows only a fixed generic
native message.

## Lifecycle and verification

The controller belongs to one native product window and is not cloneable into a
paint, protocol, or application path. It keeps its worker outside the window
registry lock; a timer poll only checks whether a worker already finished. If
the product window ends, the host does not start another update stage. A UAC
installer that Windows already started remains responsible for its own existing
transaction and recovery rules.

The automated checks prove command masking, no automatic restart wording,
controller identity validation, terminal-outcome separation, and the product
window's existing lifecycle boundaries. A desktop acceptance run still requires
a deliberately prepared newer signed fixture and catalogue: select the system
menu action, decline the Anodrel prompt once, approve it once, exercise UAC,
wait for the fixed restart-needed message, close and relaunch the product, and
verify the selected record. This is not run automatically because the fixture
changes machine certificate trust. See [development product-update fixture](PRODUCT_UPDATE_FIXTURE.md).

See [update flow](UPDATE_FLOW.md), [update consent](UPDATE_CONSENT.md),
[update delivery](UPDATE_DELIVERY.md), [update handoff](UPDATE_HANDOFF.md),
and Decision 0199.
