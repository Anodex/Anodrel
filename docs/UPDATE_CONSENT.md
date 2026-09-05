# Windows update consent

**Status:** The host-owned native confirmation and its product-window system
menu action are implemented. The host represents signed-byte progress only
through its caption/taskbar surface; automatic restart, scheduling,
suppression preferences, and application protocol exposure remain separate
work.

## Purpose

Before a host asks to download an offered update or invokes Windows UAC, it must
obtain a local decision from the person using the machine. The direct consent
adapter accepts only an opaque `AvailableUpdate` that already passed signed
catalogue discovery. It shows this fixed native confirmation:

~~~text
An update to version <signed version> is ready.

Download and install it now?
~~~

The title is `Anodrel update`. The version comes only from the CMS-verified
candidate; no application text, endpoint, path, publisher, release notes, or
installer detail is displayed.

## Native behavior

The adapter calls the direct Windows `MessageBoxW` API with only an information
icon, `Yes` and `No` buttons, and `No` as the default focused button. The
calling native host invokes it on its UI thread only after a local click on the
fixed product system-menu action. `Yes` returns the same opaque offer for the
later download stage. `No` returns an ordinary decline.

The adapter never remembers, suppresses, retries, reports, schedules, or
initiates a decision. It performs no network, cache, file, signature, UAC,
process, registry, or installation operation.

## Exclusions

This is not an application-controlled dialog, a protocol operation, a release
notes viewer, a background update prompt, a settings preference, an automatic
check, or a replacement for Windows UAC. The product system-menu integration
keeps all blocking work off its UI thread.

See [update flow](UPDATE_FLOW.md), [update acceptance](UPDATE_ACCEPTANCE.md),
and [Windows update delivery](UPDATE_DELIVERY.md), and [product updates](PRODUCT_UPDATES.md).
