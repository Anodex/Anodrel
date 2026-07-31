# Decision 0003: Establish the protocol and mock host before a native host

**Status:** Accepted

**Date:** 2026-07-31

## Context

The native host framework, UI strategy, and native transport remain open
decisions. Beginning with any one framework would expose its APIs directly to
applications and make the public platform boundary difficult to change.

## Decision

Anodrel will first ship a TypeScript workspace containing:

- a JSON-compatible, versioned protocol package;
- an application SDK that accepts an abstract transport;
- a policy-driven in-memory mock host; and
- shared protocol contract tests plus a small sample application.

The mock transport is test infrastructure only. It does not decide the native
host implementation, native transport, or webview-versus-native-UI strategy.

## Consequences

Positive:

- applications can develop and test against documented contracts now;
- a future Rust native host can be compared with the mock using the same tests;
- capability ownership and error behavior are made explicit before privileged
  operations exist.

Tradeoffs:

- TypeScript protocol definitions will need a compatible native representation;
- the mock does not validate window, process, credential, or operating-system
  behavior.

## Revisit conditions

Revisit if a native implementation proves that the JSON-compatible protocol
cannot provide the required security, performance, or interoperability.
