# Windows native update flow

**Status:** The opaque native flow, its fixed native product system-menu
action, and its bounded native progress presentation are implemented. It has
no application protocol, automatic restart, or joined signed acceptance run.

## Purpose

`anodrel-windows-updater` composes the already bounded native updater modules
without widening any one of them into a general service. A host chooses a valid
installed application identity and receives an opaque offer. It may then carry
that offer through the fixed sequence:

~~~text
fixed machine record
        |
        v
fixed current-user cache recovery
        |
        v
signed catalogue discovery and candidate preflight
        |
        v
fresh image download and locked exact-release acceptance
        |
        v
fixed Windows UAC `runas` handoff with `update`
~~~

No public method accepts an endpoint, cache root, installer image, command,
argument, path, release version, publisher, registry location, or certificate.
The values passed between stages are opaque and are consumed in order.

## Consent and outcome

The flow itself has no application protocol. The separate native consent adapter
returns the same opaque offer only after an explicit local `Yes` decision, with
`No` focused by default. A verified product window now reaches that prompt only
from its fixed host-owned **Check for Anodrel updates** system-menu action. UAC
cancellation remains a normal safe result. The returned process handle is
waited away from the UI thread; after a zero exit, the native postcondition
checks final installed policy. Only then does the host show its fixed
restart-needed message. Successful launch or exit alone remains insufficient
proof.

This intentionally does not create an application update API, automatic update
path, background service, scheduler, notification, automatic restart, cache
queue, pause, retry, speed or time estimate, or general network/file/process
API. The host-owned caption and best-effort taskbar visual report only signed
byte progress; see [product update progress](PRODUCT_UPDATE_PROGRESS.md).

See [update discovery](UPDATE_DISCOVERY.md), [update cache](UPDATE_CACHE.md),
[update delivery](UPDATE_DELIVERY.md), [update handoff](UPDATE_HANDOFF.md), and
[update consent](UPDATE_CONSENT.md), and [product updates](PRODUCT_UPDATES.md).
