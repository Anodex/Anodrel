# Packages

This directory contains reusable cross-platform packages.

Planned package categories include protocol types, client SDKs, validation,
testing utilities, and development tooling. Packages must remain independent
from any one consuming application.

The initial packages are:

- `protocol/` -- public message types and validation helpers;
- `sdk/` -- application-facing client over an abstract transport;
- `mock-host/` -- policy-driven host for local development and contract tests.
- `windows-transport/` -- development-only Node-core client for the documented
  Windows bootstrap and named-pipe frames; it is not a shipped runtime or
  content-hosting layer.
- `anodex-adapter/` -- a small migration adapter that maps Anodrel's portable
  window-state API into Anodex title-bar state. It does not import Electron or
  contain Anodex application code.

`docs/SDK.md` defines the package-root SDK surface and its compatibility rules.
