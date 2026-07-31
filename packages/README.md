# Packages

This directory contains reusable cross-platform packages.

Planned package categories include protocol types, client SDKs, validation,
testing utilities, and development tooling. Packages must remain independent
from any one consuming application.

The initial packages are:

- `protocol/` -- public message types and validation helpers;
- `sdk/` -- application-facing client over an abstract transport;
- `mock-host/` -- policy-driven host for local development and contract tests.
