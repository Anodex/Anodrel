# Continuous verification

Anodrel runs its owned verification workflow on every pull request and every
push to `main`. The workflow is intentionally split by the two source
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

## Repository policy

After the first successful run, repository administrators should require both
workflow jobs before changes enter `main`. That GitHub setting is intentionally
outside the repository: Anodrel can publish the checks, but it must not assume
authority to change a maintainer's review or merge policy.
