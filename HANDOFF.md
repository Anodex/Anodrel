# Anodrel Project Handoff

**Updated:** 2026-08-01
**Project path:** `C:\Users\Owner\Desktop\Platform X`
**Latest decision:** 0056 — credential protocol uses separate exact grants
**Protocol version:** 1.12

This file is a resume pointer, not a status report. It says where to start, how
to check the tree is healthy, and what the next gate is. It deliberately does
not restate the capability surface or the phase status, because those went
stale here once already:

- **What exists today** → README.md
- **Phase status, acceptance gates, Startup Lab tiles** → ROADMAP.md
- **Why a boundary is shaped the way it is** → `docs/decisions/`
- **Rules for changing anything** → AGENTS.md

## What this project is

Anodrel is a standalone, reusable native application platform intended to
replace Electron for applications that need desktop windows, operating-system
services, permissions, secure storage, process management, and a controlled
application/runtime boundary.

Anodex is the first planned application that may use Anodrel. Anodex is not
part of this repository and has not been modified.

The product name is Anodrel. The local workspace still uses its original folder
name.

## How to resume

Open this folder as its own workspace, then read in order:

1. HANDOFF.md (this file)
2. README.md
3. ROADMAP.md
4. AGENTS.md
5. docs/ARCHITECTURE.md
6. docs/DEVELOPMENT.md
7. docs/PROTOCOL.md, docs/TRANSPORT.md, docs/THREAT_MODEL.md
8. The decision records for the area being changed

## Verify the tree first

Before changing anything, confirm the baseline is green:

~~~text
npm run check
npm test
cd native && cargo test --workspace
~~~

As of this handoff: `npm test` passes 31 tests, `cargo test --workspace` passes
369 tests, and both build without warnings. If those numbers have dropped, find
out why before adding work.

`start.bat` builds in release and opens the Windows Startup Lab for a visual
smoke test.

## Shape of the work

Every capability so far has followed the same ladder, one commit per rung. Keep
using it:

1. Portable crate under `native/crates/` — no operating-system or third-party
   dependency, `forbid(unsafe_code)`, tested by asserting on values.
2. Windows adapter under `native/adapters/` — direct Win32, host-only.
3. Protocol grant in `packages/protocol` — a new minor version, with separate
   exact grants per operation rather than one broad capability.
4. SDK surface and mock-host policy in `packages/sdk` and `packages/mock-host`.
5. Contract test in `tests/contract` proving SDK and host agree, and that each
   operation refuses to run without its exact host-issued grant.
6. A development-only diagnostic that exercises the real path over the
   authenticated Windows pipe.
7. A numbered decision record, and the matching `docs/` contract file.

Do not claim a rung is done until its documentation and verification are done.

## Next milestone

**Signed package distribution and verified executable identity.** This is the
Phase 2 acceptance item and the gate on the `Launch Sample` Startup Lab tile —
the only tile still `Planned`. It carries the largest threat-model change in the
project so far and must not be linked before that entry exists.

The supporting pieces are already built and currently unused:

- Decision 0017 — Windows Authenticode verifier, returns a leaf certificate
  fingerprint.
- Decision 0018 — installed application record binding expected executable
  digest and publisher fingerprint to a validated package identity.
- Decision 0019 — machine-wide 64-bit registry policy reader for that record.
- Decision 0020 — launch service that locks, revalidates, verifies, and tracks
  a policy-approved executable before delivering bootstrap material.

What is missing is record provisioning and an installed sample to launch.
Sequence:

1. Provision an installed application record for the sample.
2. Bind the verified executable session to its validated application ID through
   the existing private bootstrap boundary, without exposing bootstrap material.
3. Extend the threat model, then link the tile.

Everything shipped to date is a development-only diagnostic path. There is no
product session lifecycle yet, and capability breadth is currently ahead of it.
Prefer closing this gate over adding a further capability.

## Non-negotiable boundaries

AGENTS.md holds the full rules. The ones that have no other home:

- Keep Anodrel in its own repository; do not copy Anodex source into it, and do
  not add Anodrel files to the Anodex repository.
- Do not begin a large Anodex migration before platform contracts are stable.
- Do not expose arbitrary native access to application content.

## Relationship to Anodex

Anodex remains at:

~~~text
C:\Users\Owner\Desktop\Anodex4
~~~

It builds and operates independently. Future integration should use a documented
Anodrel adapter, introduced only after the platform has a working host, contract
tests, recovery behavior, and a rollback plan.

## Keeping this file honest

Update the header block and the **Next milestone** section whenever a milestone
closes. If you find yourself adding a list of what the project now contains, put
it in README.md instead — that is the mistake this rewrite corrected.
