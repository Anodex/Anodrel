# Installed application records v1

**Status:** Windows launch-service contract. The service is internal to the
native host; it does not define a public application capability or make the
Startup Lab tile available until a provisioned signed application exists.

## Purpose and boundary

An executable and its package directory are mutable application input. An
`anodrel.application.json` file inside that directory cannot authorize its own
publisher or executable. Before a Windows host may start a product process, it
needs a separate policy record selected from a trusted host policy directory.

An installed application record binds one validated application ID to:

- a canonical package root;
- a canonical `.exe` below that root and its SHA-256 digest; and
- one approved leaf signing-certificate SHA-256 fingerprint.

The record must reside outside the package directory. A future Windows policy
store selects the policy directory and ensures only an authorized installer or
administrator can change it. Applications, package content, protocol messages,
command-line arguments, and rendered UI never choose a record or policy
directory.

This parser verifies record shape, package identity, executable containment,
and executable digest. It does **not** call Windows Authenticode, start a
process, create a pipe, or expose a record to an application or renderer.

## Record format

A record is strict UTF-8 JSON no larger than **16 KiB**. Version 1.0 accepts
the existing five fields and grants no capabilities. Version 1.1 adds the
required `capabilities` array. Version 1.2 adds the later storage, credential,
and file-operation grants without changing the version 1.1 interpretation.
Version 1.7 adds the separately scoped `file.write_text` grant defined by
Decision 0079. Version 1.8 adds the separately scoped `menu.write` grant
defined by Decision 0080. Version 1.9 adds the separately scoped
`window.focus` grant defined by Decision 0085. Version 1.10 adds the separately
scoped `window.fullscreen` grant defined by Decision 0086. Version 1.11 adds
the separately scoped `file.write_binary` grant defined by Decision 0087.
Version 1.12 adds the separately scoped `window.size` grant defined by Decision
0088.
Unknown, missing, duplicate, and wrongly typed fields are rejected.

~~~json
{
  "recordVersion": { "major": 1, "minor": 1 },
  "applicationId": "org.anodrel.sample",
  "packageRoot": "C:\\Program Files\\Anodrel\\Sample",
  "executable": {
    "path": "bin/anodrel-sample.exe",
    "sha256": "64 lowercase hexadecimal characters"
  },
  "publisher": {
    "leafCertificateSha256": "64 lowercase hexadecimal characters"
  },
  "capabilities": ["diagnostics.read", "ui.document.write", "ui.events.read"]
}
~~~

| Field | Rule |
| --- | --- |
| `recordVersion` | Object with numeric `major: 1`; minor `0` grants nothing; every later supported minor requires `capabilities`. |
| `applicationId` | Uses the same 3â€“128 character identity grammar as the validated package manifest and exactly equals its `applicationId`. |
| `packageRoot` | Absolute local directory path. Its canonical value is private host data and is never rendered. |
| `executable.path` | Relative forward-slash-separated package path. It cannot contain roots, drives, `.` or `..`, or backslashes, and must end in `.exe` (case-insensitive). The canonical result remains inside `packageRoot`. |
| `executable.sha256` | Lowercase hexadecimal SHA-256 of raw executable bytes. Files above **128 MiB** are rejected. |
| `publisher.leafCertificateSha256` | Lowercase hexadecimal SHA-256 fingerprint expected from the accepted embedded Authenticode leaf certificate. It is internal comparison data, never display text. |
| `capabilities` | Required in 1.1 and later. Exact non-duplicate supported grants selected by machine policy. 1.1 supports `diagnostics.read`, `ui.document.write`, `ui.events.read`, `session.close`, `clipboard.read`, `clipboard.write`, and `external.open`; 1.2 additionally supports `dialog.open_file`, `dialog.save_file`, `file.read_text`, `storage.state.read`, `storage.state.replace`, `storage.state.clear`, `credential.read`, `credential.write`, and `credential.delete`; 1.3 adds `notification.show`; 1.4 adds `window.title`; 1.5 adds `ui.fields.read`; 1.6 adds `window.state`; 1.7 adds `file.write_text`; 1.8 adds `menu.write`; 1.9 adds `window.focus`; 1.10 adds `window.fullscreen`; 1.11 adds `file.write_binary`; and 1.12 adds `window.size`. Each version is a strict superset of the one before, and naming a later version's grant in an earlier record is invalid. |

The package root must contain `anodrel.application.json`. The parser loads it
with normal containment and content-digest checks before accepting the record's
identity binding.

## Windows machine-policy store

The first trusted policy source is read-only and machine-wide:

~~~text
HKEY_LOCAL_MACHINE\Software\Anodrel\Applications\<applicationId>
    record    REG_SZ
~~~

The direct Windows adapter always opens the 64-bit registry view with only
`KEY_QUERY_VALUE` access. It accepts no current-user store, registry
virtualization fallback, environment-variable override, package-supplied path,
or application-supplied record. The key component must already be a valid
application ID, and the JSON field must exactly match it before package
validation begins.

`record` is UTF-16 `REG_SZ` JSON. It must contain one trailing NUL and no
embedded NUL, fit within 32 KiB of UTF-16 registry data, convert to valid Rust
text, and then meet the 16 KiB JSON record limit above. The adapter reads only;
it has no create, write, delete, registry enumeration, or installer API.

Windows normally limits writes below this `HKEY_LOCAL_MACHINE` location to
administrative installation or system management. A later installation service
must document how it provisions records and validates the key's access-control
policy. The read adapter trusts this machine-policy boundary but does not claim
that an administrator is untrusted.

The portable validator has a dedicated trusted-record entry point for this
operating-system source. It performs every package and executable check from
this document but does not accept a filesystem policy root, because the record
has already come from the machine registry. That entry point is for a native
policy adapter; it is not a public application API.

The same Windows policy adapter can convert a successfully validated record
into one host session policy through `anodrel-session-policy`. This carries
only the record's application ID and its strict machine-selected capability
array. The adapter does not create the pipe, bootstrap a child, launch a
process, or make a record visible to an application.

`anodrel-windows-registered-session` composes this derived policy with an
identity-bound service bundle and the owner-restricted Windows pipe listener.
The bundle supplies state storage, Credential Manager, bounded text clipboard,
and validated HTTPS handoff. It leaves UI-bound file and document services
unavailable until public window lifecycle is defined. It returns the listener
and its separate sensitive invitation, but does not begin I/O, launch the
executable, or deliver the invitation. The native host must still perform
locked executable and signer verification before using the private bootstrap
adapter.

For an interactive application, the same adapter can instead create one
grouped registered UI session. It binds the pipe to a host-created document
mailbox, semantic-input mailbox, close signal, UI-thread dialog mailbox, and
retained file-text service before the peer authenticates. The group can enter
only the host's internal authenticated-window entry point; it has no launch,
title, handle, or application-request API. A future launch coordinator must
still create it before bootstrap delivery, run locked executable verification,
track the child, and end both child and window on shutdown.

`anodrel-windows-product-session` now supplies that host-only coordinator.
It joins the registered interactive session, locked launch, pipe worker, and
child-exit watcher without making any of them application-facing. See
`docs/PRODUCT_SESSIONS.md` for the exact ownership and shutdown contract, and
`docs/PRODUCT_FIXTURE.md` for the development-only signed application that
exercises it.

## Compatibility and failures

Records are exact at their declared version because they influence process
authority. A compatible extension requires a new minor version, documentation,
and tests before acceptance. A breaking change requires a new major version.
Version 1.0 remains a no-grants migration format; version 1.1 accepts only its
original machine-policy grants, version 1.2 accepts the documented later grant
set, version 1.3 adds `notification.show`, and later versions add only their
documented named grants. Each version fails closed for unknown values, and for
any grant a later version introduced.

The parser fails closed if the record is outside the selected policy root,
inside the package root, malformed, oversized, mismatched with the package, or
names an executable that is missing, too large, escapes the package, or does
not match its declared digest. Failure categories never include a raw path,
certificate subject, fingerprint, argument, bootstrap invitation, or native
error. The Windows adapter also fails closed for a missing registry key,
non-string or malformed registry value, malformed UTF-16, access denial, or a
record that changes while being read.

## Windows launch sequence

`anodrel-windows-launch` is the host-only service that turns a machine-policy
record into a process launch. Before `CreateProcessW`, it runs this exact
sequence off the Win32 UI thread:

1. read the host-selected application ID from the machine policy store;
2. open the record's canonical executable with direct `CreateFileW` using read
   access and `FILE_SHARE_READ` only, preventing a new writer or delete/rename
   handle while the verification and launch are in progress;
3. recanonicalize the locked path, require it to still be the contained record
   executable, and calculate the SHA-256 digest through that same locked
   handle;
4. call the Windows Authenticode adapter while the lock remains held and match
   its leaf fingerprint to the approved record fingerprint;
5. create the exact verified `.exe` with no shell and no application arguments;
6. deliver the existing one-use bootstrap invitation only to that newly created
   child; and
7. return a tracked child handle. Dropping that handle terminates the child, so
   the native host can enforce shutdown rather than leaving a background
   process.

The service accepts an invitation prepared by the private pipe adapter but does
not create a public session or expose the invitation. Existing bootstrap code
terminates a child if invitation delivery itself fails. Any earlier failure
creates no child. The service has no restart, argument, environment, shell,
UI, logging, or application-request interface.

The registered application must be re-read by this sequence for every launch;
an earlier package validation result cannot authorize a later executable. The
file lock closes only after `CreateProcessW` has returned. This prevents a
write, delete, or rename race between digest/signature verification and Windows
opening the process image.

The same module exposes a **verification-only** entry point that runs steps 1
through 4 and then releases the lock. It creates no process, pipe, bootstrap
material, or session, and returns no path, digest, certificate value, or native
error. A host surface uses it to decide whether to offer a launch action at all;
a successful result describes that moment only, because every launch re-runs the
full sequence.

The Startup Lab tile is resolved from that preflight rather than from a
compile-time constant. It is live only while a machine record and signed
executable currently validate, and it is drawn, hovered, and hit-tested from
that one value. `docs/PRODUCT_FIXTURE.md` defines the development-only signed
application that can be provisioned today; production packaging, installation,
updates, and a real signing identity remain separate work.

~~~text
trusted Windows policy directory
        |
        v
strict installed application record
        |
        +--> canonical package + validated application identity
        +--> canonical executable + SHA-256 digest
        `--> approved signer fingerprint
                    |
                    v
        locked Authenticode comparison and tracked process launch
~~~

See `docs/SIGNING.md`, `docs/APPLICATIONS.md`, and Decisions 0018 through
0023.
