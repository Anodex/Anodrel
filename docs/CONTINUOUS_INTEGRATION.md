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

## Linux host foundations

The Ubuntu runner installs stable Rust, checks all native formatting, then
lints and tests the Linux transport, strict invitation adapter, direct
effective-account paths adapter, direct bounded state adapter, direct
host-only crash store, direct development launcher, fixed compiled child probe,
and direct Wayland lab. Its tests create real Linux abstract Unix-domain sockets, exercise the
same-UID peer check, authenticated health round trip, failed authentication
closure, and host-only stop paths, then prove a separate child process can
consume one ANLI record and complete health through the owned launcher. The
paths tests exercise the current effective account without exposing its name or
home directory. The storage tests use a real temporary filesystem to prove
atomic snapshot recovery, private modes, and link rejection. The crash-store
tests prove private record creation, retention, bounded enumeration, and link
rejection. The Wayland tests prove strict wire encoding and decoding, display
locator validation, required-global selection, fixed buffer availability,
pointer event validation, fixed click activation, and headless diagnostic-canvas
composition without adding a compositor or GUI framework. This keeps the
Linux-specific code from being treated as verified merely because the Windows
workspace compiles its intentionally empty non-Linux facade.

This job verifies only the documented Linux transport, storage, crash-record,
development-launch, fixed-child proof, and direct-Wayland diagnostic rules. It
does not start a compositor or prove a visible window or physical pointer. It
does not claim a Linux application host, identity, product launcher, SDK,
packaging, or macOS implementation.

## Repository policy

After the first successful run, repository administrators should require all
workflow jobs before changes enter `main`. That GitHub setting is intentionally
outside the repository: Anodrel can publish the checks, but it must not assume
authority to change a maintainer's review or merge policy.
