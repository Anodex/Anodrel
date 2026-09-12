# Local signed update fixture

**Status:** Contract accepted; implementation is next Windows release work.

## Purpose

The existing fixed development update-acceptance runner proves the client-side
sequence only when an operator has independently provided a newer signed image
and HTTPS catalogue. This fixture will supply those two artifacts locally so a
development machine can exercise the complete owned path without a public
endpoint, a downloaded server, or a configurable update source.

It is a development acceptance fixture, not a product update service, release
channel, web server, SDK feature, or application capability.

## Fixed scope

The fixture will use one identity distinct from the ordinary product and
no-restart fixtures: `org.anodrel.local-update-fixture`. Its initial signed
release will select only this update catalogue location:

~~~text
https://localhost:45863/anodrel/local-update/stable.p7s
~~~

The catalogue will describe only one newer signed 0.1.1 installer at:

~~~text
https://localhost:45863/anodrel/local-update/releases/0.1.1/installer.exe
~~~

The local publisher certificate signs the installed 0.1.0 image, the 0.1.1
candidate image, and the attached CMS catalogue. A separate local server TLS
certificate identifies `localhost`; it is transport trust only and cannot
substitute for the publisher-continuity or CMS checks.

## Publication boundary

An Anodrel-owned native fixture server will use the Windows HTTP Server API.
It will bind only the fixed HTTPS prefix after the elevated fixture-preparation
step has installed a matching localhost TLS binding and a URL reservation for
the current operator. The server accepts no command-line arguments or
configuration files and reads only its fixed private fixture root.

It serves exact `GET` requests for the catalogue and installer paths above. It
returns a fixed not-found response for every other method or path, accepts no
request body, writes no application data, proxies no traffic, and exposes no
directory listing, logging endpoint, shutdown endpoint, or mutable control
surface. Startup independently rechecks that the two prepared regular files
remain inside its fixed root before accepting a request.

The preparation script will create the two fresh signed installers, derive and
sign the catalogue from the locked 0.1.1 image, configure the temporary
Windows-only TLS endpoint, and register only the fixed 0.1.0 release. It will
not install the product or start the server. Normal-user commands will start
the server, invoke the separate no-argument local-fixture acceptance runner,
and stop the server. Removing the fixture will first require policy/package/cache absence,
then remove only this fixture's URL reservation, TLS binding, certificates,
and private staging directory.

## Acceptance sequence

~~~text
elevated preparation
    -> installed fixed 0.1.0 release
    -> fixed local HTTP Server API publishes signed catalogue + 0.1.1 image
    -> native Anodrel consent (No by default)
    -> direct WinHTTP HTTPS retrieval and CMS verification
    -> locked image acceptance and fixed UAC update handoff
    -> selected 0.1.1 policy proof
    -> server stop, cache retirement, and fixture removal
~~~

The operator must separately observe the native consent, UAC, and final native
result. Read-only verifiers may prove only fixed policy, package, registration,
and release-version facts; they cannot claim a dialog was seen or a restart was
unnecessary.

## Exclusions

This fixture does not add a public port, arbitrary listener, HTTP, proxy,
certificate bypass, redirects, application-selected endpoint, automatic update,
background service, scheduler, external hosting, installer repair, production
identity, timestamp, or release operation.

See [product-update fixture](PRODUCT_UPDATE_FIXTURE.md),
[update acceptance](UPDATE_ACCEPTANCE.md), and Decision 0221.
