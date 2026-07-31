# Applications

This directory contains applications that consume Anodrel.

Applications own product behavior and user experience. They must use the
documented platform SDK or protocol instead of importing native host internals.

Planned consumers include a sample application, a command-line application,
and eventually the Anodex adapter.

`sample/` is the initial SDK consumer. Its TypeScript demo uses the mock host
to show the public boundary. The directory also contains the first static
`anodrel.application.json` package and bounded text content for the direct
Windows host; that package has no executable code or native bridge.
