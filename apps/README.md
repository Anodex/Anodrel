# Applications

This directory contains applications that consume Anodrel.

Applications own product behavior and user experience. They must use the
documented platform SDK or protocol instead of importing native host internals.

The current consumers are a sample application and a separate command-line
example. The Anodex adapter remains a later integration project.

`sample/` is the initial SDK consumer. Its TypeScript demo uses the mock host
to show the public boundary. The directory also contains the first static
`anodrel.application.json` package and bounded text content for the direct
Windows host; that package has no executable code or native bridge.

`command-line/` is a small public-SDK example. It reports health and the
capabilities already issued by a mock session; it does not import a native host
or pretend to be an installed command.
