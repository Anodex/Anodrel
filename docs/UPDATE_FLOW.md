# Windows native update flow

**Status:** The opaque native flow is implemented. It does not yet provide an
application protocol, user-facing consent surface, progress reporting, restart,
or a joined signed acceptance run.

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

The flow itself has no application protocol or visual interface. A future host
surface must obtain explicit user intent before it requests download or UAC.
UAC cancellation remains a normal safe result. The returned process handle can
be waited away from a UI thread; after a zero exit, the native postcondition
checks the final installed policy. Successful launch or exit alone remains
insufficient proof.

This intentionally does not create an automatic update path, a background
service, a scheduler, a notification, a restart, a cache queue, or a general
network/file/process API.

See [update discovery](UPDATE_DISCOVERY.md), [update cache](UPDATE_CACHE.md),
[update delivery](UPDATE_DELIVERY.md), and [update handoff](UPDATE_HANDOFF.md).
