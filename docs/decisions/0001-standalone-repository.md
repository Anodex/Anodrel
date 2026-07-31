# Decision 0001: Anodrel lives in its own repository

**Status:** Accepted

**Date:** 2026-07-31

## Context

Anodrel is intended to become a reusable application platform. Anodex is an
existing application with its own Git history, release process, architecture,
and product-specific behavior.

Building Anodrel inside Anodex would mix two lifecycles and make it harder
to reuse the platform for other applications. It would also make an eventual
Electron-to-native migration harder to review and roll back.

## Decision

Anodrel is developed at:

~~~text
C:\Users\Owner\Desktop\Platform X
~~~

It has its own Git repository and documentation. Anodex remains in its existing
repository and is not modified as part of the initial foundation work.

## Consequences

Positive:

- clean Git history and ownership boundaries;
- reusable platform independent of Anodex;
- safer experimentation before migration;
- easier future applications and adapters;
- clear review of the eventual Anodex integration.

Tradeoffs:

- some interfaces may temporarily be represented in both repositories;
- integration testing will eventually span two repositories;
- changes must be coordinated through documented protocol versions.

## Revisit conditions

Revisit this decision only if the platform becomes permanently specific to
Anodex. Until then, repository separation is the default rule.
