# Cross-component tests

This directory contains tests that exercise more than one package or host
boundary.

Unit tests should live with their package. Contract, protocol, host integration,
security, and end-to-end tests may live here when they span components.

`contract/` currently verifies the public protocol against the mock host. A
future native host must run the same assertions for every implemented operation.
