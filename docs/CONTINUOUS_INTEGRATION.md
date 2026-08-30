# Continuous verification

Anodrel runs its owned verification workflow on every pull request and every
push to `main`. The workflow is intentionally split by the three source
boundaries the repository ships today.

## TypeScript SDK and contracts

The Windows runner installs the exact locked Node dependencies, then runs the
full TypeScript type check and the public SDK, mock-host, transport, sample,
command-line, and protocol-contract tests. This catches a change where the
public TypeScript surface and the test host no longer agree.

## Native Windows host and documentation

The Windows runner installs stable Rust, then checks formatting, runs Clippy as
an error, and executes the full native workspace test suite. It also enforces
the repository's 550-line maintained-source limit and validates every local
link in `docs/` and the project site source.

This automation exercises code and documentation only. It does not replace the
documented manual Windows checks for visual rendering, foreground policy,
native dialogs, screen-reader speech, or the development product fixture. A
green workflow must never be used to claim those acceptance checks passed.

## Linux local transport

The Ubuntu runner installs stable Rust, checks all native formatting, then
lints and tests the Linux transport, strict invitation adapter, and fixed
compiled child probe. Its tests create real Linux abstract Unix-domain sockets,
exercise the same-UID peer check, authenticated health round trip, failed
authentication closure, and host-only stop paths, then prove a separate child
process can consume one ANLI record and complete health. This keeps the
Linux-specific code from being treated as verified merely because the Windows
workspace compiles its intentionally empty non-Linux facade.

This job verifies only the documented Linux transport and fixed-child proof. It
does not claim a Linux native window, reusable launcher, application SDK,
packaging, or any macOS implementation.

## Repository policy

After the first successful run, repository administrators should require all
workflow jobs before changes enter `main`. That GitHub setting is intentionally
outside the repository: Anodrel can publish the checks, but it must not assume
authority to change a maintainer's review or merge policy.
