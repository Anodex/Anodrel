# Development product-update acceptance fixture

**Status:** The fixed-identity acceptance runner is implemented. A newer signed
fixture release, its HTTPS catalogue, and a temporary trusted development
certificate are still required for a successful manual run.

## Purpose

`anodrel-product-update-acceptance` exercises Anodrel's native Windows update
sequence for exactly `org.anodrel.product-fixture`. It accepts no arguments and
does not read application, package, endpoint, path, certificate, or update data
from the command line, environment, or a rendered surface. The installed signed
policy remains the only source of update location and release facts.

It is an operator-only development diagnostic, not a product host, a Startup
Lab action, an application capability, or an update SDK. The normal product
host now keeps discovery, transfer, waiting, and restart-needed presentation
behind its explicit native UI-thread and worker boundaries; see
[product updates](PRODUCT_UPDATES.md).

## Fixed sequence

~~~text
fixed fixture identity
          |
          v
signed-policy discovery and private-cache recovery
          |
          v
native Anodrel confirmation (No by default)
          |
          v
private image transfer, lock, and signature acceptance
          |
          v
Windows UAC confirmation and fixed `update` command
          |
          v
process observation and installed-policy postcondition proof
~~~

The diagnostic performs no automatic check, retry, schedule, restart, rollback,
notification, or preference write. Declining either the Anodrel confirmation or
Windows UAC is a distinct ordinary outcome. Only a zero installer exit followed
by the postcondition proof reports verification.

## Operator boundary

Invoke the compiled diagnostic with no arguments only after a development
fixture is deliberately installed with a newer signed release and a valid fixed
HTTPS catalogue source. It is safe for an unprepared machine to refuse before
any download or elevation. A complete positive run needs the same temporary
machine trust change documented for the [product fixture](PRODUCT_FIXTURE.md).

The diagnostic has no product window. Its consent prompt runs on its initial
interactive thread; its later transfer and process wait happen on that command
runner rather than a product UI thread. That split is appropriate only for this
manual acceptance tool and must not be copied into an interactive host.

See [update acceptance](UPDATE_ACCEPTANCE.md), [update flow](UPDATE_FLOW.md),
and Decision 0174.
