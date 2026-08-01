# Decision 0042: External links start as validated HTTPS handoff

**Status:** Accepted

**Date:** 2026-08-01

## Context

Opening a link is useful but a general shell or URI launcher could also open a
file, invoke a custom protocol handler, or interpret application-controlled
arguments. That would bypass Anodrel's explicit operating-system boundary.

## Decision

The first external-link value is a bounded ASCII `https://` address with a
strict DNS-style authority parser. The direct Windows adapter passes only that
validated value to `ShellExecuteW` without a verb, arguments, working
directory, child process, or callback. It returns one safe unavailable category
and records neither the address nor a native error.

## Consequences

- applications can later request an ordinary browser handoff without shell
  string construction;
- files, commands, and custom URI handlers cannot enter this service; and
- protocol capability exposure, consent, and broader schemes remain explicit
  future decisions.

## Revisit conditions

Revisit before adding HTTP, another scheme, non-ASCII hostnames, a browser or
handler selector, a result callback, a user prompt, or a protocol operation.
