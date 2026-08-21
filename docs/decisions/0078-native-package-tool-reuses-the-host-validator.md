# Decision 0078: Native package tooling reuses the host validator

**Status:** Accepted

**Date:** 2026-08-21

## Context

The first PowerShell starter wrapper made the current text-package boundary
easy to try on Windows, but it had to repeat identity, text-limit, digest, and
manifest-writing rules. A copied authoring implementation can drift from the
portable validator that the Windows host actually trusts.

The project also needs first-party developer tooling that does not depend on a
packaging framework, browser runtime, or Node process. The current package
format is portable Rust plus the standard library, so its authoring tool can be
as well.

## Decision

`anodrel-package-tool` is a small native workspace binary with two exact
commands: `init` creates one new `anodrel.text.v1` package and `verify` loads
one existing package. It delegates identity and text validation, SHA-256, exact
manifest formation, containment checks, and post-write verification to
`anodrel-application`, the same first-party module the host uses.

`init` accepts only a destination, application ID, display name, and optional
plain text. It normalises line endings before validation, refuses an existing
destination, writes UTF-8 content and manifest plus the package-local Git
attribute rule, then verifies the package it just wrote. `verify` prints only
the validated identity and content facts; it never prints raw content.

The existing PowerShell command remains a thin Windows convenience wrapper. It
does no package validation or digest calculation itself; it invokes the native
tool with the same arguments.

Neither command installs, signs, launches, grants, registers, updates, or
executes an application.

## Consequences

- The authoring command and native host agree through one owned validator,
  rather than matching duplicate implementations by convention.
- Package creation and verification work wherever the Rust workspace builds;
  the PowerShell wrapper remains optional Windows ergonomics.
- The tool is still only for the current static content package, not a
  production packager or executable application scaffold.

## Revisit conditions

Revisit when a new content format, signed package, executable launcher,
published SDK template, installer, or updater is introduced. Each must retain
an explicit validator and its own secure authoring and verification workflow.
