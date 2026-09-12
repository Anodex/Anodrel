# Decision 0219: Signed uninstall cleanup uses a private verified handoff

**Status:** Accepted

**Date:** 2026-09-11

**Supersedes:** Decision 0197's restart-delayed final cleanup.

## Context

The original fixed uninstaller retains its own running signed image and asks
Windows to delete that image and its empty package directory at restart. That
is safe, but makes an ordinary uninstall require a reboot before a same-version
development install or fixture-trust cleanup can proceed.

An external cleanup process must not become a general deletion utility. In
particular, its command line, environment, current directory, registry input,
or arbitrary file cannot choose a target. The selected policy is intentionally
removed during uninstall, so a later process cannot use it as its only proof.

## Decision

The owned signed installer will add one fixed cleanup mode and one private
handoff protocol. The elevated selected uninstaller creates a separately
located, byte-for-byte verified copy of its current signed image in a
machine-owned private cleanup stage. It starts that image with only the fixed
`cleanup` command and supplies its target through one inherited private stream;
no cleanup target appears in a command line, environment variable, registry
value, or predictable file.

Before the elevated parent removes selected policy or package content, the
helper independently reads the still-selected machine record and proves:

- its inherited cleanup description names that exact selected package;
- the selected executable and both signed installer images have the selected
  publisher; and
- the selected package and uninstaller paths are canonical, ordinary,
  installer-derived locations.

The helper acknowledges that proof privately. Only after that acknowledgement
does the parent remove the Start-menu and Apps & features registrations, remove
selected policy, and exit. The helper then removes only normal selected-package
entries, waits for the original selected installer process to exit, removes its
fixed remaining image and directories, and finally removes its own private
stage. A cleanup failure is a safe incomplete-removal result; it never falls
back to caller-selected deletion or silently reports success.

## Consequences

- Ordinary successful uninstall can complete without a Windows restart.
- The helper is first-party, signed, bounded, and independently verified.
- The release path gains a second signed image copy and a small private
  handoff protocol, both requiring unit and installed-fixture acceptance tests.
- Restart-delayed deletion remains unavailable as a silent fallback; a failure
  stays visible for recovery rather than leaving ambiguous cleanup state.

## Revisit conditions

Revisit for a separately versioned helper binary, a Windows package identity,
a machine service, per-user installation, multiple channels, repair, or
another operating system.
