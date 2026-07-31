# Applications

This directory contains applications that consume Anodrel.

Applications own product behavior and user experience. They must use the
documented platform SDK or protocol instead of importing native host internals.

Planned consumers include a sample application, a command-line application,
and eventually the Anodex adapter.

`sample/` is the initial SDK consumer. It uses the mock host only to show the
public boundary; it is not a native application host.
