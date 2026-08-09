# Development Windows product fixture

**Status:** Development-only Windows verification fixture. This is **not** a
public SDK surface, an application capability, a packaging format, an installer,
or an update mechanism. It exists so the verified product-session coordinator
defined in `docs/PRODUCT_SESSIONS.md` can be exercised end to end on a developer
machine.

## Why a fixture exists

`anodrel-windows-product-session` joins a machine-policy record, a locked and
signer-verified child, an authenticated named-pipe worker, and one host-owned
native window. Every one of those boundaries is already implemented and tested
in isolation, but the joined path has never run, because the repository ships no
signed executable and no machine policy record.

A fixture closes that gap without inventing a product. It supplies exactly three
things:

1. a first-party child executable that speaks the existing authenticated
   protocol and nothing else;
2. a staged package directory holding that executable and a valid
   `anodrel.application.json`; and
3. one machine-policy record under the existing
   `HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>` location.

Nothing about the fixture changes the host's trust rules. The host still reads
machine policy, still locks and revalidates the executable, still calls Windows
Authenticode, still compares the approved publisher fingerprint, and still
delivers the invitation only over the child-only bootstrap handle.

## Fixture identity

| Value | Fixture |
| --- | --- |
| Application ID | `org.anodrel.product-fixture` |
| Display name | `Anodrel Product Fixture` |
| Executable | `bin/anodrel-product-fixture.exe` |
| Record version | `1.2` |
| Grants | `ui.document.write`, `ui.events.read`, `session.close` |

The identity is deliberately distinct from `org.anodrel.sample`, so provisioning
the fixture cannot silently redirect the existing package or Startup Lab
identity. The grant set is the smallest one that can prove a native window round
trip: no clipboard, link, dialog, file, storage, credential, or diagnostics
grant is requested, so a mistake in the fixture cannot reach those services.

## Staged package layout

The provisioning helper stages a package directory. It is ordinary application
input and carries no authority by itself.

~~~text
<staging root>/
├── anodrel.application.json     # strict manifest, validated by anodrel-application
├── content/
│   └── main.txt                 # bounded anodrel.text.v1 content
└── bin/
    └── anodrel-product-fixture.exe   # first-party, Authenticode-signed
~~~

The record lives **outside** this directory, in the machine registry. The
existing record parser rejects a record that resolves inside its own package
root, so a fixture package cannot authorize itself.

## Fixture child behaviour

The child is `anodrel-product-fixture.exe`. It has one job: prove the
authenticated path and then exit. In order it

1. reads exactly one `ANBI` bootstrap record from its inherited standard input
   and closes that channel;
2. connects to the named pipe named by that record;
3. sends the authentication control message built from the invitation;
4. calls `platform.capabilities` and requires exactly the three grants its
   machine record declares — this is both its liveness check and its proof that
   the record's capability array reached the authenticated session, and it needs
   no grant of its own;
5. calls `ui.document.replace` with one compiled-in
   `anodrel.ui.document.v1` document and requires revision `1`;
6. polls `ui.events.read` until the host-rendered semantic action
   `fixture.session.action` arrives for that same revision;
7. calls `session.close`; and
8. exits `0`.

It has no command line, no configuration file, no environment input, no network
access, no filesystem write, and no console output. The host passes it no
arguments; `docs/LAUNCH.md` forbids that.

### Waiting on a person

Step 6 waits for a human, and every poll is a real round trip that wakes the
host's pipe worker, drains a mailbox, and encodes a response. The fixture
therefore backs off rather than polling at a fixed rate: intervals start at
25 ms, grow by half each time, and cap at one second, with the whole wait
bounded at two minutes.

A click in the first moment is still answered within a few tens of milliseconds,
and the worst case a person can experience is the one-second cap. Over the full
two minutes that is **128 round trips instead of the 1,200** a fixed 100 ms
interval would make — an idle product window should not cost a constant stream
of IPC.

### Fixture exit stages

The bootstrap launcher redirects child output to `NUL`, so an exit code is the
only signal. Codes name a boundary, never a cause:

| Code | Stage |
| --- | --- |
| `0` | the full round trip completed |
| `11` | the bootstrap record could not be read or decoded |
| `12` | the named pipe could not be opened |
| `13` | authentication did not complete |
| `14` | the session did not carry exactly the record's declared grants |
| `15` | `ui.document.replace` was rejected or returned another revision |
| `16` | `ui.events.read` failed, dropped, or discarded a candidate |
| `17` | the expected semantic action did not arrive before the wait bound |
| `18` | `session.close` was not accepted |

No code carries a path, invitation, token, certificate value, or Windows error.
Every code stays below 32, so none of them can be confused with the `0xA11D`
exit the launch service uses when it terminates a child during host shutdown.

## Signing and machine trust

The fixture is signed with a **development** code-signing certificate created on
the developer's own machine by Windows PowerShell's `New-SelfSignedCertificate`
and applied by `Set-AuthenticodeSignature`. Both ship with Windows; the fixture
introduces no third-party signing tool and no third-party runtime.

Windows Authenticode accepts the signature only if the certificate chains to a
trusted root. The provisioning script therefore installs the generated
certificate into `LocalMachine\Root` and `LocalMachine\TrustedPublisher`.

**This is a real machine trust change and is the fixture's largest cost.** It is
acceptable only on a development machine, only for a certificate whose private
key that machine generated, and only while the fixture is in use. The
provisioning script has a matching removal mode that deletes the record, the
staged package, and both certificate entries. `docs/THREAT_MODEL.md` records this
as a development-environment assumption, not a production control.

The host does not participate in any of this. It never creates a certificate,
never installs trust, never writes the registry, and never signs anything.

## Provisioning contract

Provisioning is a two-part operation and both parts are outside the host:

1. `scripts/provision-product-fixture.ps1` — Windows tooling only. It builds the
   fixture and the helper with Cargo, stages the package, creates or reuses the
   development certificate, signs the executable, installs machine trust, and
   then calls the helper. Removal reverses every step. Provisioning and removal
   require elevation; a separate `-Verify` switch reports the current state as a
   query only and needs none.
2. `anodrel-product-provisioning` — a first-party Rust helper. Given the staged
   package root it recomputes the executable's SHA-256 digest, asks the existing
   Windows Authenticode adapter for the accepted leaf certificate fingerprint,
   composes the strict record defined in `docs/LAUNCH.md`, validates that record
   through the same parser the host uses, and only then writes the single
   `record` value under the machine policy key.

The helper is the only component in this repository that writes machine policy.
It is a development tool, is not linked into the native host, and requires an
elevated shell because Windows protects that key. It exposes exactly three
operations — `provision`, `remove`, and `verify` — and no way to name a registry
path, value name, or hive.

A record the helper composes is rejected before it is written if the package,
manifest, containment, digest, or identity checks fail. The helper never writes
a record it could not itself validate.

## Host activation

Two host-only entry points consume the fixture. Neither is reachable from an
application, a package, a protocol message, or rendered content.

### `--product-session <applicationId>`

The direct route. The host validates the identity, starts
`start_registered_product_session` on a worker thread, waits for that worker,
and only then runs the authenticated native window on its own UI thread with the
session's grouped UI resources. When the window returns it calls `finish`, which
requests shutdown and joins both workers.

The application ID comes from the local command line of the host binary, which
is operator input on a development machine — the same trust level as
`--ui-preview`. It selects *which already-provisioned machine record to read*; it
cannot supply a record, a path, an executable, or a capability.

### The Startup Lab launch tile

The Startup Lab resolves the tile's state at startup by running a
**verification-only** preflight: read the machine record, lock the executable
against write, delete, and rename, recanonicalize and rehash it through that
lock, call Windows Authenticode, and compare the approved publisher fingerprint.
The preflight creates no process, no pipe, and no bootstrap material.

On a provisioned machine it is the most expensive check the host runs before its
window exists — a full executable hash plus an Authenticode chain evaluation
that may reach revocation infrastructure — so it starts on a worker as soon as
the host owns the surface and runs beside the core health check and the private
pipe loopback. It is joined immediately before window creation, because the
tile's state must be settled before the surface opens: that is what lets drawing
and hit-testing share one value instead of a tile that changes under the
pointer. A preflight that could not be started, or that stopped unexpectedly,
answers "not launchable"; an unavailable check never widens what the surface
offers.

The tile is drawn and hit-tested from that one resolved value:

- preflight failed, or no record is provisioned → the tile stays **planned**,
  dimmed, and inert, exactly as before;
- preflight succeeded → the tile becomes **linked** and starts one product
  session.

Because drawing and hit-testing read the same value, the tile cannot be made
live by changing how it looks, and it cannot be live on a machine where the
record and signature do not currently validate. The tile shows no path,
certificate, fingerprint, or Windows failure detail; an unavailable fixture is
reported only as the existing planned state.

Clicking a linked tile starts the coordinator on a worker thread. The UI thread
opens the product window only after that worker reports success, and it ends the
session when that window is destroyed.

## What the fixture deliberately is not

- It is not a product, an installer, or an update path. Nothing here creates
  Program Files content, a service, a shortcut, a scheduled task, or an
  uninstall entry.
- It is not a public SDK. No package, protocol message, or application API can
  provision, select, launch, inspect, or terminate a fixture session.
- It is not evidence of Electron parity. It proves one authenticated window
  round trip for one child, not application packaging, multi-window policy,
  restart, background execution, or crash recovery.
- It is not a production trust story. A self-signed development certificate
  installed into a local machine store is a test harness, not a publisher
  identity.

## Verification

Automated coverage lives with each component:

- the fixture's document, action name, and stage codes are unit tested without
  Windows;
- the provisioning helper's record composition is unit tested, along with its
  refusals: an unsigned executable, an absent executable, a record whose digest
  no longer matches its package, and a record offered under another
  application's identity. `provision` reaches the registry only through a
  successful composition, so each refusal means nothing was written;
- `anodrel-windows-launch` has a verification-only entry point with tests that
  it fails closed on an unprovisioned machine; and
- the Startup Lab has a test that the launch tile is linked only when a
  preflight result says the fixture validated.

The joined path needs a real machine. `docs/DEVELOPMENT.md` carries the manual
sequence: provision, run the host route, confirm the delivered document,
activate the action, watch the window close, and confirm the child is gone.

See `docs/LAUNCH.md`, `docs/PRODUCT_SESSIONS.md`, `docs/SIGNING.md`, and
Decisions 0017 through 0020, 0058 through 0060, and 0061.
